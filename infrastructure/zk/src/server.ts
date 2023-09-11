import { Command } from 'commander';
import * as utils from './utils';
import * as env from './env';
import { clean } from './clean';
import fs from 'fs';

export async function server(rebuildTree: boolean, openzeppelinTests: boolean, components?: string) {
    let options = '';
    if (openzeppelinTests) {
        options += '--features=openzeppelin_tests';
    }
    if (rebuildTree || components) {
        options += ' --';
    }
    if (rebuildTree) {
        clean('db');
        options += ' --rebuild-tree';
    }
    if (components) {
        options += ` --components=${components}`;
    }
    await utils.spawn(`cargo run --bin micro_server --release ${options}`);
}

export async function externalNode() {
    if (process.env.MICRO_ENV != 'ext-node') {
        console.warn(`WARINING: using ${process.env.MICRO_ENV} environment for external node`);
        console.warn('If this is a mistake, set $MICRO_ENV to "ext-node" or other environment');
    }
    await utils.spawn('cargo run --release --bin micro_external_node');
}

async function create_genesis(cmd: string) {
    await utils.confirmAction();
    await utils.spawn(`${cmd} | tee genesis.log`);
    const genesisContents = fs.readFileSync('genesis.log').toString().split('\n');
    const genesisBlockCommitment = genesisContents.find((line) => line.includes('CONTRACTS_GENESIS_BLOCK_COMMITMENT='));
    const genesisBootloaderHash = genesisContents.find((line) => line.includes('CHAIN_STATE_KEEPER_BOOTLOADER_HASH='));
    const genesisDefaultAAHash = genesisContents.find((line) => line.includes('CHAIN_STATE_KEEPER_DEFAULT_AA_HASH='));
    const genesisRoot = genesisContents.find((line) => line.includes('CONTRACTS_GENESIS_ROOT='));
    const genesisRollupLeafIndex = genesisContents.find((line) =>
        line.includes('CONTRACTS_GENESIS_ROLLUP_LEAF_INDEX=')
    );
    if (genesisRoot == null || !/^CONTRACTS_GENESIS_ROOT=0x[a-fA-F0-9]{64}$/.test(genesisRoot)) {
        throw Error(`Genesis is not needed (either Postgres DB or tree's Rocks DB is not empty)`);
    }

    if (
        genesisBootloaderHash == null ||
        !/^CHAIN_STATE_KEEPER_BOOTLOADER_HASH=0x[a-fA-F0-9]{64}$/.test(genesisBootloaderHash)
    ) {
        throw Error(`Genesis is not needed (either Postgres DB or tree's Rocks DB is not empty)`);
    }

    if (
        genesisDefaultAAHash == null ||
        !/^CHAIN_STATE_KEEPER_DEFAULT_AA_HASH=0x[a-fA-F0-9]{64}$/.test(genesisDefaultAAHash)
    ) {
        throw Error(`Genesis is not needed (either Postgres DB or tree's Rocks DB is not empty)`);
    }

    if (
        genesisBlockCommitment == null ||
        !/^CONTRACTS_GENESIS_BLOCK_COMMITMENT=0x[a-fA-F0-9]{64}$/.test(genesisBlockCommitment)
    ) {
        throw Error(`Genesis is not needed (either Postgres DB or tree's Rocks DB is not empty)`);
    }

    if (
        genesisRollupLeafIndex == null ||
        !/^CONTRACTS_GENESIS_ROLLUP_LEAF_INDEX=([1-9]\d*|0)$/.test(genesisRollupLeafIndex)
    ) {
        throw Error(`Genesis is not needed (either Postgres DB or tree's Rocks DB is not empty)`);
    }

    const date = new Date();
    const [year, month, day, hour, minute, second] = [
        date.getFullYear(),
        date.getMonth(),
        date.getDate(),
        date.getHours(),
        date.getMinutes(),
        date.getSeconds()
    ];
    const label = `${process.env.MICRO_ENV}-Genesis_gen-${year}-${month}-${day}-${hour}${minute}${second}`;
    fs.mkdirSync(`logs/${label}`, { recursive: true });
    fs.copyFileSync('genesis.log', `logs/${label}/genesis.log`);
    env.modify('CONTRACTS_GENESIS_ROOT', genesisRoot);
    env.modify('CHAIN_STATE_KEEPER_BOOTLOADER_HASH', genesisBootloaderHash);
    env.modify('CHAIN_STATE_KEEPER_DEFAULT_AA_HASH', genesisDefaultAAHash);
    env.modify('CONTRACTS_GENESIS_BLOCK_COMMITMENT', genesisBlockCommitment);
    env.modify('CONTRACTS_GENESIS_ROLLUP_LEAF_INDEX', genesisRollupLeafIndex);
}

export async function genesisFromSources() {
    await create_genesis('cargo run --bin micro_server --release -- --genesis');
}

export async function genesisFromBinary() {
    await create_genesis('micro_server --genesis');
}

export const serverCommand = new Command('server')
    .description('start micro server')
    .option('--genesis', 'generate genesis data via server')
    .option('--rebuild-tree', 'rebuilds merkle tree from database logs', 'rebuild_tree')
    .option('--openzeppelin-tests', `enables 'openzeppelin_tests' feature`)
    .option('--components <components>', 'comma-separated list of components to run')
    .action(async (cmd: Command) => {
        if (cmd.genesis) {
            await genesisFromSources();
        } else {
            await server(cmd.rebuildTree, cmd.openzeppelinTests, cmd.components);
        }
    });

export const enCommand = new Command('external-node').description('start micro external node').action(async () => {
    await externalNode();
});
