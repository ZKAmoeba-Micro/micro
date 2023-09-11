import { BigNumber, BytesLike, Contract } from 'ethers';
import { ethers } from 'ethers';
import { Provider, types, utils } from 'micro-web3';
import { Deployer } from '@zkamoeba/hardhat-micro-deploy';
import { hashBytecode } from 'micro-web3/build/src/utils';
import { expect } from 'chai';
import * as hre from 'hardhat';

const L1_CONTRACTS_FOLDER = `${process.env.MICRO_HOME}/contracts/ethereum/artifacts/cache/solpp-generated-contracts`;
const DIAMOND_UPGRADE_INIT_ABI = new ethers.utils.Interface(
    require(`${L1_CONTRACTS_FOLDER}/micro/upgrade-initializers/DiamondUpgradeInit1.sol/DiamondUpgradeInit1.json`).abi
);
const DIAMOND_CUT_FACET_ABI = new ethers.utils.Interface(
    require(`${L1_CONTRACTS_FOLDER}/micro/facets/DiamondCut.sol/DiamondCutFacet.json`).abi
);


export interface ForceDeployment {
    // The bytecode hash to put on an address
    bytecodeHash: BytesLike;
    // The address on which to deploy the bytecodehash to
    newAddress: string;
    // The value with which to initialize a contract
    value: BigNumber;
    // The constructor calldata
    input: BytesLike;
    // Whether to call the constructor
    callConstructor: boolean;
}

export function diamondCut(facetCuts: any[], initAddress: string, initCalldata: string): any {
    return {
        facetCuts,
        initAddress,
        initCalldata
    };
}


export async function deployOnAnyLocalAddress(
    ethProvider: ethers.providers.Provider,
    l2Provider: Provider,
    deployments: ForceDeployment[],
    factoryDeps: BytesLike[]
): Promise<string> {
    const diamondUpgradeInitAddress = process.env.CONTRACTS_DIAMOND_UPGRADE_INIT_ADDR;

    // The same mnemonic as in the etc/test_config/eth.json
    const govMnemonic = require('../../../test_config/constant/eth.json').mnemonic;

    if (!diamondUpgradeInitAddress) {
        throw new Error('DIAMOND_UPGRADE_INIT_ADDRESS not set');
    }

    const govWallet = ethers.Wallet.fromMnemonic(govMnemonic, "m/44'/60'/0'/0/1").connect(ethProvider);

    const microContract = await l2Provider.getMainContractAddress();

    const micro = new ethers.Contract(microContract, utils.MICRO_MAIN_ABI, govWallet);

    // In case there is some pending upgrade there, we cancel it
    const upgradeProposalState = await micro.getUpgradeProposalState();
    if (upgradeProposalState != 0) {
        const currentProposalHash = await micro.getProposedUpgradeHash();
        await micro.connect(govWallet).cancelUpgradeProposal(currentProposalHash);
    }

    // Encode data for the upgrade call
    const encodedParams = utils.CONTRACT_DEPLOYER.encodeFunctionData('forceDeployOnAddresses', [deployments]);

    // Prepare the diamond cut data
    const upgradeInitData = DIAMOND_UPGRADE_INIT_ABI.encodeFunctionData('forceDeployL2Contract', [
        encodedParams,
        factoryDeps,
        parseInt(process.env.CONTRACTS_PRIORITY_TX_MAX_GAS_LIMIT as string)
    ]);

    const upgradeParam = diamondCut([], diamondUpgradeInitAddress, upgradeInitData);
    const currentProposalId = (await micro.getCurrentProposalId()).add(1);
    // Get transaction data of the `proposeTransparentUpgrade`
    const proposeTransparentUpgrade = DIAMOND_CUT_FACET_ABI.encodeFunctionData('proposeTransparentUpgrade', [
        upgradeParam,
        currentProposalId
    ]);

    // Get transaction data of the `executeUpgrade`
    const executeUpgrade = DIAMOND_CUT_FACET_ABI.encodeFunctionData('executeUpgrade', [
        upgradeParam,
        ethers.constants.HashZero
    ]);

    // Proposing the upgrade
    await (
        await govWallet.sendTransaction({
            to: microContract,
            data: proposeTransparentUpgrade
        })
    ).wait();

    // Finalize the upgrade
    const receipt = await (
        await govWallet.sendTransaction({
            to: microContract,
            data: executeUpgrade
        })
    ).wait();

    return utils.getL2HashFromPriorityOp(receipt, microContract);
}

export async function deployContractOnAddress(
    name: string,
    address: string,
    input: BytesLike,
    deployer: Deployer,
): Promise<Contract> {
    const artifact = await deployer.loadArtifact(name);
    const bytecodeHash = hashBytecode(artifact.bytecode);

    const factoryDeps = [
        artifact.bytecode,
        ...await deployer.extractFactoryDeps(artifact)
    ];

    const deployment: ForceDeployment = {
        bytecodeHash,
        newAddress: address,
        value: BigNumber.from(0),
        input,
        callConstructor: true
    };

    const txHash = await deployOnAnyLocalAddress(
        deployer.ethWallet.provider,
        deployer.zkWallet.provider,
        [deployment],
        factoryDeps
    )

    const receipt = await deployer.zkWallet.provider.waitForTransaction(txHash);

    expect(receipt.status, 'Contract deployment failed').to.eq(1);

    return new ethers.Contract(
        address,
        artifact.abi,
        deployer.zkWallet.provider
    );
}
