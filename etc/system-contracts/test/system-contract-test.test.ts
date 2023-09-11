import { Wallet, utils } from "micro-web3";
import * as hre from "hardhat";
import { Deployer } from "@zkamoeba/hardhat-micro-deploy";

import { TestSystemContract } from "../typechain/TestSystemContract";
import { deployContractOnAddress } from "./utils/deployOnAnyAddress";
import { BigNumber, ethers } from "ethers";

const RICH_WALLET_PK = '0x0ee20f526d623cfa645ddc4183425ab8a89a101dbf3d9f698539ac33412ecbee';

describe('System contracts tests', function () {
    // An example address where our system contracts will be put
    const TEST_SYSTEM_CONTRACT_ADDRESS = '0x0000000000000000000000000000000000000101';
    let testContract: TestSystemContract;
    let deployer = new Deployer(hre, new Wallet(RICH_WALLET_PK));

    before('Prepare bootloader and system contracts', async function () {
        testContract = (await deployContractOnAddress(
            'TestSystemContract',
            TEST_SYSTEM_CONTRACT_ADDRESS,
            "0x",
            deployer
        )).connect(deployer.zkWallet) as TestSystemContract;

        await (await deployer.zkWallet.deposit({
            token: utils.ETH_ADDRESS,
            amount: ethers.utils.parseEther('10.0')
        })).wait();
    });

    it('Test precompile call', async function () {
        await testContract.testPrecompileCall();
    })

    it('Test mimicCall and setValueForNextCall', async function () {
        const whoToMimic = Wallet.createRandom().address;
        const value = BigNumber.from(2).pow(128).sub(1);
        await (await testContract.testMimicCallAndValue(
            whoToMimic,
            value
        ));
    });

    it('Test onlySystemCall modifier', async function () {
        await testContract.testOnlySystemModifier();
    });

    it('Test system mimicCall', async function () {
        await testContract.testSystemMimicCall();
    });
});
