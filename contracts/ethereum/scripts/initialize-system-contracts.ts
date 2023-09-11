import { Command } from 'commander';
import { ethers, Wallet } from 'ethers';
import { Deployer } from '../src.ts/deploy';
import { formatUnits, parseUnits } from 'ethers/lib/utils';
import { web3Provider, getNumberFromEnv, REQUIRED_L2_GAS_PRICE_PER_PUBDATA } from './utils';

import * as fs from 'fs';
import * as path from 'path';

const provider = web3Provider();
const testConfigPath = path.join(process.env.MICRO_HOME as string, `etc/test_config/constant`);
const ethTestConfig = JSON.parse(fs.readFileSync(`${testConfigPath}/eth.json`, { encoding: 'utf-8' }));

const systemContractArtifactsPath = path.join(process.env.MICRO_HOME as string, 'etc/system-contracts/artifacts-zk/');

const systemCrtifactsPath = path.join(systemContractArtifactsPath, 'cache-zk/solpp-generated-contracts/');

const contractArtifactsPath = path.join(process.env.MICRO_HOME as string, 'contracts/ethereum/artifacts/');

const tokenArtifactsPath = path.join(contractArtifactsPath, 'cache/solpp-generated-contracts/micro');

const l2TokenAdress = process.env.CONTRACTS_L2_ZKAT_ADDR;

function readInterface(path: string, fileName: string) {
    const abi = JSON.parse(fs.readFileSync(`${path}/${fileName}.sol/${fileName}.json`, { encoding: 'utf-8' })).abi;
    return new ethers.utils.Interface(abi);
}

const DEPOSIT_INTERFACE = readInterface(systemCrtifactsPath, 'Deposit');
const PROOF_REWARD_POOL_INTERFACE = readInterface(systemCrtifactsPath, 'ProofRewardPool');
const ZKA_INTERFACE = readInterface(tokenArtifactsPath, 'ZKAmoebaToken');

const DEPOSIT_ADDRESS = '0x0000000000000000000000000000000000008100';
const PROOF_ADDRESS = '0x0000000000000000000000000000000000008102';

async function main() {
    const program = new Command();

    program.version('0.1.0').name('initialize-system-contracts');

    program
        .option('--private-key <private-key>')
        .option('--gas-price <gas-price>')
        .option('--owner <owner>')
        .option('--token <token>')
        .option('--pakage-cycle <pakage-cycle>')
        .option('--min-deposit-amount <min-deposit-amount>')
        .option('--release-cycle <release-cycle>')
        .option('--penalize-ratio <penalize-ratio>')
        .option('--confiscation-vote-ratio <confiscation-vote-ratio>')
        .option('--confiscation-to-node-percent <confiscation-to-node-percent>')
        .option('--proof-time-target <proof-time-target>')
        .option('--adjustment-quotient <adjustment-quotient>')
        .action(async (cmd) => {
            const deployWallet = cmd.privateKey
                ? new Wallet(cmd.privateKey, provider)
                : Wallet.fromMnemonic(
                      process.env.MNEMONIC ? process.env.MNEMONIC : ethTestConfig.mnemonic,
                      "m/44'/60'/0'/0/0"
                  ).connect(provider);
            console.log(`Using deployer wallet: ${deployWallet.address}`);

            const gasPrice = cmd.gasPrice ? parseUnits(cmd.gasPrice, 'gwei') : await provider.getGasPrice();
            console.log(`Using gas price: ${formatUnits(gasPrice, 'gwei')} gwei`);

            const deployer = new Deployer({
                deployWallet,
                governorAddress: deployWallet.address,
                verbose: true
            });

            const micro = deployer.microContract(deployWallet);
            const priorityTxMaxGasLimit = getNumberFromEnv('CONTRACTS_PRIORITY_TX_MAX_GAS_LIMIT');

            const owner = cmd.owner ? cmd.owner : deployWallet.address;
            const token = cmd.token ? cmd.token : l2TokenAdress;
            const pakageCycle = cmd.pakageCycle ? parseInt(cmd.pakageCycle) : 0;
            const minDepositAmount = cmd.minDepositAmount
                ? parseUnits(cmd.minDepositAmount, 'ether')
                : parseUnits('100', 'ether');
            const releaseCycle = cmd.releaseCycle ? parseInt(cmd.releaseCycle) : 0;
            const penalizeRatio = cmd.penalizeRatio ? parseInt(cmd.penalizeRatio) : 300;
            const confiscationVoteRatio = cmd.confiscationVoteRatio ? parseInt(cmd.confiscationVoteRatio) : 300;
            const confiscationToNodePercent = cmd.confiscationToNodePercent
                ? parseInt(cmd.confiscationToNodePercent)
                : 0;
            const proofTimeTarget = cmd.proofTimeTarget ? parseInt(cmd.proofTimeTarget) : 150;
            const adjustmentQuotient = cmd.adjustmentQuotient ? parseInt(cmd.adjustmentQuotient) : 100;

            const requiredValueToPublishBytecodes = await micro.l2TransactionBaseCost(
                gasPrice,
                priorityTxMaxGasLimit,
                REQUIRED_L2_GAS_PRICE_PER_PUBDATA
            );

            const depositInitializationParams = DEPOSIT_INTERFACE.encodeFunctionData('initialize', [
                owner,
                token,
                pakageCycle,
                minDepositAmount,
                releaseCycle,
                penalizeRatio,
                confiscationVoteRatio,
                confiscationToNodePercent
            ]);

            const depositInitializaTx = await micro.requestL2Transaction(
                DEPOSIT_ADDRESS,
                0,
                depositInitializationParams,
                priorityTxMaxGasLimit,
                REQUIRED_L2_GAS_PRICE_PER_PUBDATA,
                [],
                deployWallet.address,
                { gasPrice, value: requiredValueToPublishBytecodes, gasLimit: 1_000_000_000 }
            );
            const depositInitializaRecepit = await depositInitializaTx.wait();
            console.log(`Deposit initialized, gasUsed: ${depositInitializaRecepit.gasUsed.toString()}`);

            const proofRewardPoolInitializationParams = PROOF_REWARD_POOL_INTERFACE.encodeFunctionData('initialize', [
                owner,
                token,
                proofTimeTarget,
                adjustmentQuotient
            ]);
            const proofTx = await micro.requestL2Transaction(
                PROOF_ADDRESS,
                0,
                proofRewardPoolInitializationParams,
                priorityTxMaxGasLimit,
                REQUIRED_L2_GAS_PRICE_PER_PUBDATA,
                [],
                deployWallet.address,
                { gasPrice, value: requiredValueToPublishBytecodes, gasLimit: 1_000_000_000 }
            );
            const proofReceipt = await proofTx.wait();

            console.log(`ProofRewardPool initialized, gasUsed: ${proofReceipt.gasUsed.toString()}`);

            //approve and deposit
            const depositAmount = parseUnits('1000000', 'ether');
            const tokenApproveParams = ZKA_INTERFACE.encodeFunctionData('approve', [DEPOSIT_ADDRESS, depositAmount]);

            const approveTx = await micro.requestL2Transaction(
                token,
                0,
                tokenApproveParams,
                priorityTxMaxGasLimit,
                REQUIRED_L2_GAS_PRICE_PER_PUBDATA,
                [],
                deployWallet.address,
                { gasPrice, value: requiredValueToPublishBytecodes, gasLimit: 1_000_000_000 }
            );
            const approveRecepit = await approveTx.wait();
            console.log(`Token approve, gasUsed: ${approveRecepit.gasUsed.toString()}`);

            const depositParams = DEPOSIT_INTERFACE.encodeFunctionData('deposit', [depositAmount]);

            const depositTx = await micro.requestL2Transaction(
                DEPOSIT_ADDRESS,
                0,
                depositParams,
                priorityTxMaxGasLimit,
                REQUIRED_L2_GAS_PRICE_PER_PUBDATA,
                [],
                deployWallet.address,
                { gasPrice, value: requiredValueToPublishBytecodes, gasLimit: 1_000_000_000 }
            );
            const depositRecepit = await depositTx.wait();
            console.log(`Deposit deposit, gasUsed: ${depositRecepit.gasUsed.toString()}`);

            //transfer to m/44'/60'/0'/0/2  999000000
            const receiveAddress = Wallet.fromMnemonic(
                process.env.MNEMONIC ? process.env.MNEMONIC : ethTestConfig.mnemonic,
                "m/44'/60'/0'/0/2"
            ).address;
            const transferAmount = parseUnits('999000000', 'ether');
            const tokenTransferParams = ZKA_INTERFACE.encodeFunctionData('transfer', [receiveAddress, transferAmount]);

            const transferTx = await micro.requestL2Transaction(
                token,
                0,
                tokenTransferParams,
                priorityTxMaxGasLimit,
                REQUIRED_L2_GAS_PRICE_PER_PUBDATA,
                [],
                deployWallet.address,
                { gasPrice, value: requiredValueToPublishBytecodes, gasLimit: 1_000_000_000 }
            );
            const transferRecepit = await transferTx.wait();
            console.log(`Token transfer, gasUsed: ${transferRecepit.gasUsed.toString()}`);
        });

    await program.parseAsync(process.argv);
}

main()
    .then(() => process.exit(0))
    .catch((err) => {
        console.error('Error:', err);
        process.exit(1);
    });
