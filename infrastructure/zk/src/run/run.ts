import { Command } from 'commander';
import * as utils from '../utils';
import { Wallet } from 'ethers';
import fs from 'fs';
import * as path from 'path';
import * as dataRestore from './data-restore';

export { dataRestore };

export async function deployERC20(command: 'dev' | 'new', name?: string, symbol?: string, decimals?: string) {
    if (command == 'dev') {
        await utils.spawn(`yarn --silent --cwd contracts/ethereum deploy-erc20 add-multi '
            [
                { "name": "DAI",  "symbol": "DAI",  "decimals": 18 },
                { "name": "wBTC", "symbol": "wBTC", "decimals":  8, "implementation": "RevertTransferERC20" }
            ]' > ./etc/tokens/localhost.json`);
    } else if (command == 'new') {
        await utils.spawn(
            `yarn --silent --cwd contracts/ethereum deploy-erc20 add --token-name ${name} --symbol ${symbol} --decimals ${decimals}`
        );
    }
}

export async function tokenInfo(address: string) {
    await utils.spawn(`yarn l1-contracts token-info info ${address}`);
}

// installs all dependencies and builds our js packages
export async function yarn() {
    await utils.spawn('yarn');
    await utils.spawn('yarn init-build');
}

export async function deployTestkit(genesisRoot: string) {
    await utils.spawn(`yarn l1-contracts deploy-testkit --genesis-root ${genesisRoot}`);
}

export async function plonkSetup(powers?: number[]) {
    if (!powers) {
        powers = [20, 21, 22, 23, 24, 25, 26];
    }
    const URL = 'https://storage.googleapis.com/universal-setup';
    fs.mkdirSync('keys/setup', { recursive: true });
    process.chdir('keys/setup');
    for (let power = 20; power <= 26; power++) {
        if (!fs.existsSync(`setup_2^${power}.key`)) {
            await utils.spawn(`curl -LO ${URL}/setup_2^${power}.key`);
            await utils.sleep(1);
        }
    }
    process.chdir(process.env.MICRO_HOME as string);
}

export async function revertReason(txHash: string, web3url?: string) {
    await utils.spawn(`yarn l1-contracts ts-node scripts/revert-reason.ts ${txHash} ${web3url || ''}`);
}

export async function explorer() {
    await utils.spawn('yarn explorer serve');
}

export async function exitProof(...args: string[]) {
    await utils.spawn(`cargo run --example generate_exit_proof --release -- ${args.join(' ')}`);
}

export async function catLogs(exitCode?: number) {
    utils.allowFailSync(() => {
        console.log('\nSERVER LOGS:\n', fs.readFileSync('server.log').toString());
        console.log('\nPROVER LOGS:\n', fs.readFileSync('dummy_prover.log').toString());
    });
    if (exitCode !== undefined) {
        process.exit(exitCode);
    }
}

export async function testAccounts() {
    const testConfigPath = path.join(process.env.MICRO_HOME as string, `etc/test_config/constant`);
    const ethTestConfig = JSON.parse(fs.readFileSync(`${testConfigPath}/eth.json`, { encoding: 'utf-8' }));
    const NUM_TEST_WALLETS = 10;
    const baseWalletPath = "m/44'/60'/0'/0/";
    const walletKeys = [];
    for (let i = 0; i < NUM_TEST_WALLETS; ++i) {
        const ethWallet = Wallet.fromMnemonic(ethTestConfig.test_mnemonic as string, baseWalletPath + i);
        walletKeys.push({
            address: ethWallet.address,
            privateKey: ethWallet.privateKey
        });
    }
    console.log(JSON.stringify(walletKeys, null, 4));
}

export async function loadtest(...args: string[]) {
    console.log(args);
    await utils.spawn(`cargo run --release --bin loadnext -- ${args.join(' ')}`);
}

export async function readVariable(address: string, contractName: string, variableName: string, file?: string) {
    if (file === undefined)
        await utils.spawn(
            `yarn --silent --cwd contracts/ethereum read-variable read ${address} ${contractName} ${variableName}`
        );
    else
        await utils.spawn(
            `yarn --silent --cwd contracts/ethereum read-variable read ${address} ${contractName} ${variableName} -f ${file}`
        );
}

export const command = new Command('run').description('run miscellaneous applications').addCommand(dataRestore.command);

command.command('test-accounts').description('print ethereum test accounts').action(testAccounts);
command.command('explorer').description('run micro explorer locally').action(explorer);
command.command('yarn').description('install all JS dependencies').action(yarn);
command.command('cat-logs [exit_code]').description('print server and prover logs').action(catLogs);

command
    .command('deploy-erc20 <dev|new> [name] [symbol] [decimals]')
    .description('deploy ERC20 tokens')
    .action(async (command: string, name?: string, symbol?: string, decimals?: string) => {
        if (command != 'dev' && command != 'new') {
            throw new Error('only "dev" and "new" subcommands are allowed');
        }
        await deployERC20(command, name, symbol, decimals);
    });

command
    .command('token-info <address>')
    .description('get symbol, name and decimals parameters from token')
    .action(async (address: string) => {
        await tokenInfo(address);
    });

command
    .command('plonk-setup [powers]')
    .description('download missing keys')
    .action(async (powers?: string) => {
        const powersArray = powers
            ?.split(' ')
            .map((x) => parseInt(x))
            .filter((x) => !Number.isNaN(x));
        await plonkSetup(powersArray);
    });

command
    .command('deploy-testkit')
    .description('deploy testkit contracts')
    .requiredOption('--genesis-root <hash>')
    .action(async (cmd: Command) => {
        await deployTestkit(cmd.genesisRoot);
    });

command
    .command('revert-reason <tx_hash> [web3_url]')
    .description('get the revert reason for ethereum transaction')
    .action(revertReason);

command
    .command('exit-proof')
    .option('--account <id>')
    .option('--token <id>')
    .option('--help')
    .description('generate exit proof')
    .action(async (cmd: Command) => {
        if (!cmd.account || !cmd.token) {
            await exitProof('--help');
        } else {
            await exitProof('--account_id', cmd.account, '--token', cmd.token);
        }
    });

command
    .command('loadtest [options...]')
    .description('run the loadtest')
    .allowUnknownOption()
    .action(async (options: string[]) => {
        await loadtest(...options);
    });

command
    .command('read-variable <address> <contractName> <variableName>')
    .option(
        '-f --file <file>',
        'file with contract source code(default $MICRO_HOME/contracts/contracts/${contractName}.sol)'
    )
    .description('Read value of contract variable')
    .action(async (address: string, contractName: string, variableName: string, cmd: Command) => {
        await readVariable(address, contractName, variableName, cmd.file);
    });
