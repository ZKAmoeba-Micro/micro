import fs from 'fs';
import { Command } from 'commander';
import { spawn } from 'zk/build/utils';
import { getFacetCutsFileName, getFacetsFileName, getUpgradePath } from '../utils';
import { callFacetDeployer } from './deployer';

async function deployAllFacets(
    l1RpcProvider: string,
    privateKey: string,
    gasPrice: string,
    create2Address: string,
    nonce: string,
    environment: string
) {
    const file = getFacetsFileName(environment);
    await callFacetDeployer(l1RpcProvider, privateKey, gasPrice, create2Address, nonce, true, true, true, true, file);
}

async function deployFacetsAndMergeFiles(
    l1RpcProvider: string,
    privateKey: string,
    gasPrice: string,
    create2Address: string,
    nonce: string,
    executor: boolean,
    admin: boolean,
    getters: boolean,
    mailbox: boolean,
    environment
) {
    create2Address = create2Address ?? process.env.CONTRACTS_CREATE2_FACTORY_ADDR;
    const upgradePath = getUpgradePath(environment);
    const tmpFacetsFile = `${upgradePath}/tmp.json`;
    await callFacetDeployer(
        l1RpcProvider,
        privateKey,
        gasPrice,
        create2Address,
        nonce,
        executor,
        admin,
        getters,
        mailbox,
        tmpFacetsFile
    );
    const tmpFacets = JSON.parse(fs.readFileSync(tmpFacetsFile).toString());
    const facetsFile = getFacetsFileName(environment);
    if (!fs.existsSync(facetsFile)) {
        fs.writeFileSync(facetsFile, JSON.stringify({}, null, 2));
    }
    const facets = JSON.parse(fs.readFileSync(facetsFile).toString());
    for (const key in tmpFacets) {
        facets[key] = tmpFacets[key];
    }
    fs.writeFileSync(facetsFile, JSON.stringify(facets, null, 4));
    fs.unlinkSync(tmpFacetsFile);
}

async function generateFacetCuts(l1RpcProvider?: string, microAddress?: string, environment?: string) {
    microAddress = microAddress ?? process.env.CONTRACTS_DIAMOND_PROXY_ADDR;

    console.log('Generating facet cuts');
    const file = getFacetsFileName(environment);
    const facets = JSON.parse(fs.readFileSync(file).toString());
    let gettersAddress = facets['GettersFacet'];
    if (gettersAddress) {
        gettersAddress = gettersAddress['address'];
    }
    let adminAddress = facets['AdminFacet'];
    if (adminAddress) {
        adminAddress = adminAddress['address'];
    }
    let mailboxAddress = facets['MailboxFacet'];
    if (mailboxAddress) {
        mailboxAddress = mailboxAddress['address'];
    }
    let executorAddress = facets['ExecutorFacet'];
    if (executorAddress) {
        executorAddress = executorAddress['address'];
    }

    await callGenerateFacetCuts(
        microAddress,
        getFacetCutsFileName(environment),
        l1RpcProvider,
        adminAddress,
        gettersAddress,
        mailboxAddress,
        executorAddress
    );
}

async function callGenerateFacetCuts(
    microAddress: string,
    file: string,
    l1RpcProvider?: string,
    adminAddress?: string,
    gettersAddress?: string,
    mailboxAddress?: string,
    executorAddress?: string
) {
    const cwd = process.cwd();
    process.chdir(`${process.env.MICRO_HOME}/contracts/ethereum/`);
    let argsString = '';
    if (l1RpcProvider) {
        argsString += ` --l1Rpc ${l1RpcProvider}`;
    }
    if (adminAddress) {
        argsString += ` --admin-address ${adminAddress}`;
    }
    if (gettersAddress) {
        argsString += ` --getters-address ${gettersAddress}`;
    }
    if (mailboxAddress) {
        argsString += ` --mailbox-address ${mailboxAddress}`;
    }
    if (executorAddress) {
        argsString += ` --executor-address ${executorAddress}`;
    }

    argsString += ` --microAddress ${microAddress}`;
    argsString += ` --file ${file}`;
    await spawn(`yarn upgrade-system facets generate-facet-cuts ${argsString}`);
    process.chdir(cwd);
}

async function deployAllFacetsAndGenerateFacetCuts(
    l1RpcProvider: string,
    privateKey: string,
    gasPrice: string,
    create2Address: string,
    microAddress: string,
    nonce: string,
    environment: string
) {
    console.log('Deploying all facets');
    create2Address = create2Address ?? process.env.CONTRACTS_CREATE2_FACTORY_ADDR;
    microAddress = microAddress ?? process.env.CONTRACTS_DIAMOND_PROXY_ADDR;

    await deployAllFacets(l1RpcProvider, privateKey, gasPrice, create2Address, nonce, environment);
    await generateFacetCuts(l1RpcProvider, microAddress, environment);
    console.log('Done');
}

export const command = new Command('facets').description('Deploy facets and generate facet cuts');

command
    .command('deploy-all')
    .description('Deploy all facets')
    .option('--private-key <private-key>')
    .option('--l1rpc <l1Rpc>')
    .option('--gas-price <gas-price>')
    .option('--nonce <nonce>')
    .option('--create2-address <create2Address>')
    .option('--micro-address <microAddress>')
    .option('--environment <environment>')
    .action(async (cmd) => {
        await deployAllFacetsAndGenerateFacetCuts(
            cmd.l1rpc,
            cmd.privateKey,
            cmd.gasPrice,
            cmd.create2Address,
            cmd.microAddress,
            cmd.nonce,
            cmd.environment
        );
    });

command
    .command('deploy')
    .description('deploy facets one by one')
    .option('--environment <environment>')
    .option('--private-key <private-key>')
    .option('--create2-address <create2Address>')
    .option('--gas-price <gas-price>')
    .option('--nonce <nonce>')
    .option('--l1rpc <l1Rpc>')
    .option('--executor')
    .option('--admin')
    .option('--getters')
    .option('--mailbox')
    .action(async (cmd) => {
        await deployFacetsAndMergeFiles(
            cmd.l1Rpc,
            cmd.privateKey,
            cmd.gasPrice,
            cmd.create2Address,
            cmd.nonce,
            cmd.executor,
            cmd.admin,
            cmd.getters,
            cmd.mailbox,
            cmd.environment
        );
    });

command
    .command('generate-facet-cuts')
    .description('Generate facet cuts')
    .option('--l1rpc <l1Rpc>')
    .option('--micro-address <microAddress>')
    .option('--environment <environment>')
    .action(async (cmd) => {
        try {
            await generateFacetCuts(cmd.l1rpc, cmd.microAddress, cmd.environment);
        } catch (e) {
            console.error('Not all facets have been deployed: ', e);
            process.exit(1);
        }
    });
