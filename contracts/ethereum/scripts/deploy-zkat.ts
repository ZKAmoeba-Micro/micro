import * as hardhat from 'hardhat';
import '@nomiclabs/hardhat-ethers';
import { Command } from 'commander';
import { Wallet } from 'ethers';
import { ethers } from 'hardhat';
import { web3Provider, REQUIRED_L2_GAS_PRICE_PER_PUBDATA, getNumberFromEnv } from './utils';
import { Deployer } from '../src.ts/deploy';
import { ZKAmoebaTokenFactory } from '../typechain/ZKAmoebaTokenFactory';
import * as fs from 'fs';
import * as path from 'path';
import { ZKAmoebaToken } from '../typechain';

const testConfigPath = path.join(process.env.MICRO_HOME as string, `etc/test_config/constant`);
const ethTestConfig = JSON.parse(fs.readFileSync(`${testConfigPath}/eth.json`, { encoding: 'utf-8' }));
const ZERO_ADDRESS = ethers.constants.AddressZero;

const provider = web3Provider();
const wallet = Wallet.fromMnemonic(ethTestConfig.mnemonic, "m/44'/60'/0'/0/0").connect(provider);

async function deployToken(): Promise<string> {
    console.log(wallet.address);
    const tokenFactory = await hardhat.ethers.getContractFactory('ZKAmoebaToken', wallet);
    const erc20 = await tokenFactory.deploy(wallet.address, { gasLimit: 500000000 });
    const receipt = await erc20.deployTransaction.wait();
    console.log(receipt.contractAddress);

    const token: ZKAmoebaToken = ZKAmoebaTokenFactory.connect(receipt.contractAddress, wallet);

    const deployer = new Deployer({
        deployWallet: wallet,
        governorAddress: ZERO_ADDRESS,
        verbose: true
    });

    const l1Bridge = deployer.defaultERC20Bridge(wallet);

    const totalSupply = await token.totalSupply();

    await token.approve(l1Bridge.address, totalSupply);

    const micro = deployer.microContract(wallet);
    const priorityTxMaxGasLimit = getNumberFromEnv('CONTRACTS_PRIORITY_TX_MAX_GAS_LIMIT');
    const requiredValueToPublishBytecodes = await micro.l2TransactionBaseCost(
        await provider.getGasPrice(),
        priorityTxMaxGasLimit,
        REQUIRED_L2_GAS_PRICE_PER_PUBDATA
    );

    const tx = await l1Bridge['deposit(address,address,uint256,uint256,uint256,address)'](
        wallet.address,
        token.address,
        totalSupply,
        priorityTxMaxGasLimit,
        REQUIRED_L2_GAS_PRICE_PER_PUBDATA,
        wallet.address,
        { gasLimit: 500000000, value: requiredValueToPublishBytecodes }
    );
    await tx.wait();

    console.log(`CONTRACTS_L1_ZKAT_ADDR=${token.address}`);

    const l2TokenAddress = await l1Bridge.l2TokenAddress(token.address);
    console.log(`CONTRACTS_L2_ZKAT_ADDR=${l2TokenAddress}`);

    return l2TokenAddress;
}

async function main() {
    const program = new Command();

    program.version('0.1.0').name('deploy-zkat').description('deploy zkat token');

    program
        .command('deploy')
        .description('deploy ZKAmoebaToken and deposit to l2')
        .action(async (cmd) => {
            let l2TokenAddress = await deployToken();
        });

    await program.parseAsync(process.argv);
}

main()
    .then(() => process.exit(0))
    .catch((err) => {
        console.error('Error:', err.message || err);
        process.exit(1);
    });

function sleep(millis: number) {
    return new Promise((resolve) => setTimeout(resolve, millis));
}
