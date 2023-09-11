import { TestMaster } from '../../src/index';
import * as micro from 'micro-web3';
import * as ethers from 'ethers';
import fetch from 'node-fetch';
import fs from 'fs';
import {
    anyTransaction,
    deployContract,
    getContractSource,
    getTestContract,
    waitForNewL1Batch
} from '../../src/helpers';
import { sleep } from 'micro-web3/build/src/utils';
import { IERC20MetadataFactory } from 'micro-web3/build/typechain';
import { extractFee } from '../../src/modifiers/balance-checker';
import { Token } from '../../src/types';

const contracts = {
    counter: getTestContract('Counter'),
    customAccount: getTestContract('CustomAccount'),
    create: {
        ...getTestContract('Import'),
        factoryDep: getTestContract('Foo').bytecode
    }
};

// Regular expression to match 32-byte hashes.
const HASH_REGEX = /^0x[\da-fA-F]{64}$/;
// Regular expression to match 20-byte addresses in lowercase.
const ADDRESS_REGEX = /^0x[\da-f]{40}$/;
// Regular expression to match variable-length hex number.
const HEX_VALUE_REGEX = /^0x[\da-fA-F]*$/;
// Regular expression to match ISO dates.
const DATE_REGEX = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{6})?/;

const ZKSOLC_VERSION = 'v1.3.10';
const SOLC_VERSION = '0.8.16';

describe('Tests for the Explorer API', () => {
    let testMaster: TestMaster;
    let alice: micro.Wallet;
    let erc20: Token;

    beforeAll(() => {
        testMaster = TestMaster.getInstance(__filename);
        alice = testMaster.mainAccount();
        erc20 = testMaster.environment().erc20Token;
    });

    test('Should test /network_stats endpoint', async () => {
        const initialStats = await query('/network_stats');
        expect(initialStats).toEqual({
            last_sealed: expect.any(Number),
            last_verified: expect.any(Number),
            total_transactions: expect.any(Number)
        });
    });

    test('Should test /blocks endpoint', async () => {
        // To ensure that the newest block is not verified yet, we're sending a transaction.
        await anyTransaction(alice);

        const blocksResponse = await query('/blocks', { direction: 'older', limit: '1' });
        expect(blocksResponse).toHaveLength(1);
        const apiBlock = blocksResponse[0];
        expect(apiBlock).toEqual({
            number: expect.any(Number),
            l1TxCount: expect.any(Number),
            l2TxCount: expect.any(Number),
            hash: expect.stringMatching(/^0x[\da-fA-F]{64}$/),
            status: expect.stringMatching(/sealed|verified/),
            timestamp: expect.any(Number)
        });

        // Sanity checks for the values we can't control.
        expect(apiBlock.l1TxCount).toBeGreaterThanOrEqual(0);
        expect(apiBlock.l2TxCount).toBeGreaterThanOrEqual(0);
        expectTimestampToBeSane(apiBlock.timestamp);

        // Retrieve block details through web3 API and cross-check the root hash.
        const blockHash = await alice.provider.getBlock(apiBlock.number).then((block) => block.hash);
        expect(apiBlock.hash).toEqual(blockHash);

        // Now try to find the same block using the "newer" query.
        const newBlocksResponse = await query('/blocks', {
            from: (apiBlock.number - 1).toString(),
            direction: 'newer',
            limit: '1'
        });
        expect(newBlocksResponse).toHaveLength(1);
        const apiBlockCopy = newBlocksResponse[0];
        // Response should be the same.
        expect(apiBlockCopy).toEqual(apiBlock);

        // Finally, in the long mode also check, that once block becomes finalized, status also changes
        // in the explorer API.
        if (!testMaster.isFastMode()) {
            await waitFor(async () => {
                const verifiedApiBlock = (
                    await query('/blocks', { from: (apiBlock.number - 1).toString(), direction: 'newer', limit: '1' })
                )[0];
                return verifiedApiBlock.status == 'verified';
            }, 'Block was not verified');
        }
    });

    test('Should test /l1_batches endpoint', async () => {
        if (testMaster.isFastMode()) {
            // This test requires a new L1 batch to be created, which may be very time consuming on stage.
            return;
        }

        // To ensure that the newest batch is not verified yet, we're sealing a new batch.
        await waitForNewL1Batch(alice);

        const l1BatchesResponse = await query('/l1_batches', { direction: 'older', limit: '1' });
        expect(l1BatchesResponse).toHaveLength(1);
        const apiL1Batch = l1BatchesResponse[0];
        expect(apiL1Batch).toMatchObject({
            number: expect.any(Number),
            l1TxCount: expect.any(Number),
            l2TxCount: expect.any(Number),
            status: expect.stringMatching(/sealed|verified/),
            timestamp: expect.any(Number)
        });

        // Sanity checks for the values we can't control.
        expect(apiL1Batch.l1TxCount).toBeGreaterThanOrEqual(0);
        expect(apiL1Batch.l2TxCount).toBeGreaterThanOrEqual(0);
        expectTimestampToBeSane(apiL1Batch.timestamp);

        // Now try to find the same batch using the "newer" query.
        const newL1BatchesResponse = await query('/l1_batches', {
            from: (apiL1Batch.number - 1).toString(),
            direction: 'newer',
            limit: '1'
        });
        expect(newL1BatchesResponse).toHaveLength(1);
        const apiL1BatchCopy = newL1BatchesResponse[0];
        // Response should be the same.
        expect(apiL1BatchCopy).toEqual(apiL1Batch);

        // Finally, in the long mode also check, that once l1 batch becomes finalized, status also changes
        // in the explorer API.
        if (!testMaster.isFastMode()) {
            await waitFor(async () => {
                const verifiedApiL1Batch = (
                    await query('/l1_batches', {
                        from: (apiL1Batch.number - 1).toString(),
                        direction: 'newer',
                        limit: '1'
                    })
                )[0];
                return verifiedApiL1Batch.status == 'verified';
            }, 'L1 batch was not verified');
        }
    });

    test('Should test /block endpoint', async () => {
        // Send the transaction to query block data about.
        const tx = await anyTransaction(alice);

        const apiBlock = await query(`/block/${tx.blockNumber}`);
        expect(apiBlock).toMatchObject({
            number: expect.any(Number),
            l1BatchNumber: expect.any(Number),
            l1TxCount: expect.any(Number),
            l2TxCount: expect.any(Number),
            rootHash: expect.stringMatching(HASH_REGEX),
            status: expect.stringMatching(/sealed|verified/),
            timestamp: expect.any(Number),
            baseSystemContractsHashes: {
                bootloader: expect.stringMatching(HASH_REGEX),
                default_aa: expect.stringMatching(HASH_REGEX)
            },
            l1GasPrice: expect.any(Number),
            l2FairGasPrice: expect.any(Number),
            operatorAddress: expect.stringMatching(/^0x[\da-f]{40}$/)
        });
        expect(apiBlock.number).toEqual(tx.blockNumber);
        expect(apiBlock.rootHash).toEqual(tx.blockHash);
        expect(apiBlock.l1TxCount).toBeGreaterThanOrEqual(0);
        expect(apiBlock.l2TxCount).toBeGreaterThanOrEqual(1); // We know that at least 1 tx is included there.
        expectTimestampToBeSane(apiBlock.timestamp);

        // Perform L1-related checks in the long mode only.
        if (!testMaster.isFastMode()) {
            // Check that L1 transaction count can also be non-zero.
            const l1Tx = await alice.deposit({ token: micro.utils.ETH_ADDRESS, amount: 1 }).then((tx) => tx.wait());
            const apiBlockWithL1Tx = await query(`/block/${l1Tx.blockNumber}`);
            expect(apiBlockWithL1Tx.l1TxCount).toBeGreaterThanOrEqual(1);

            // Wait until the block is verified and check that the required fields are set.
            let verifiedBlock = null;
            await waitFor(async () => {
                verifiedBlock = await query(`/block/${tx.blockNumber}`);
                return verifiedBlock.status == 'verified';
            }, 'Block was not verified');
            expect(verifiedBlock).toEqual({
                number: expect.any(Number),
                l1BatchNumber: expect.any(Number),
                l1TxCount: expect.any(Number),
                l2TxCount: expect.any(Number),
                rootHash: expect.stringMatching(/^0x[\da-fA-F]{64}$/),
                status: 'verified',
                timestamp: expect.any(Number),
                commitTxHash: expect.stringMatching(HASH_REGEX),
                committedAt: expect.stringMatching(DATE_REGEX),
                proveTxHash: expect.stringMatching(HASH_REGEX),
                provenAt: expect.stringMatching(DATE_REGEX),
                executeTxHash: expect.stringMatching(HASH_REGEX),
                executedAt: expect.stringMatching(DATE_REGEX),
                baseSystemContractsHashes: {
                    bootloader: expect.stringMatching(HASH_REGEX),
                    default_aa: expect.stringMatching(HASH_REGEX)
                },
                l1GasPrice: expect.any(Number),
                l2FairGasPrice: expect.any(Number),
                operatorAddress: expect.stringMatching(/^0x[\da-f]{40}$/)
            });
        }
    });

    test('Should test /l1_batch endpoint', async () => {
        if (testMaster.isFastMode()) {
            // This test requires a new L1 batch to be created, which may be very time consuming on stage.
            return;
        }

        // Send the transaction to query l1 batch data about.
        const tx = await waitForNewL1Batch(alice);

        const apiL1Batch = await query(`/l1_batch/${tx.l1BatchNumber}`);
        expect(apiL1Batch).toMatchObject({
            number: expect.any(Number),
            l1TxCount: expect.any(Number),
            l2TxCount: expect.any(Number),
            status: expect.stringMatching(/sealed|verified/),
            timestamp: expect.any(Number),
            baseSystemContractsHashes: {
                bootloader: expect.stringMatching(HASH_REGEX),
                default_aa: expect.stringMatching(HASH_REGEX)
            },
            l1GasPrice: expect.any(Number),
            l2FairGasPrice: expect.any(Number)
        });
        expect(apiL1Batch.number).toEqual(tx.l1BatchNumber);
        expect(apiL1Batch.l1TxCount).toBeGreaterThanOrEqual(0);
        expect(apiL1Batch.l2TxCount).toBeGreaterThanOrEqual(1); // We know that at least 1 tx is included there.
        expectTimestampToBeSane(apiL1Batch.timestamp);

        // Check that L1 transaction count can also be non-zero.
        const l1Tx = await alice.deposit({ token: micro.utils.ETH_ADDRESS, amount: 1 }).then((tx) => tx.wait());
        // Wait for l1 batch to be sealed.
        await waitForNewL1Batch(alice);
        const l1TxReceipt = await alice.provider.getTransactionReceipt(l1Tx.transactionHash);

        const l1BatchWithL1Tx = await query(`/l1_batch/${l1TxReceipt.l1BatchNumber}`);
        expect(l1BatchWithL1Tx.l1TxCount).toBeGreaterThanOrEqual(1);

        // Wait until the block is verified and check that the required fields are set.
        let verifiedL1Batch = null;
        await waitFor(async () => {
            verifiedL1Batch = await query(`/l1_batch/${tx.l1BatchNumber}`);
            return verifiedL1Batch.status == 'verified';
        }, 'Block was not verified');
        expect(verifiedL1Batch).toEqual({
            number: expect.any(Number),
            l1TxCount: expect.any(Number),
            l2TxCount: expect.any(Number),
            rootHash: expect.stringMatching(/^0x[\da-fA-F]{64}$/),
            status: 'verified',
            timestamp: expect.any(Number),
            commitTxHash: expect.stringMatching(HASH_REGEX),
            committedAt: expect.stringMatching(DATE_REGEX),
            proveTxHash: expect.stringMatching(HASH_REGEX),
            provenAt: expect.stringMatching(DATE_REGEX),
            executeTxHash: expect.stringMatching(HASH_REGEX),
            executedAt: expect.stringMatching(DATE_REGEX),
            baseSystemContractsHashes: {
                bootloader: expect.stringMatching(HASH_REGEX),
                default_aa: expect.stringMatching(HASH_REGEX)
            },
            l1GasPrice: expect.any(Number),
            l2FairGasPrice: expect.any(Number)
        });
    });

    test('Should test /account endpoint for an EOA', async () => {
        // Check response for the empty account.
        const newEoa = testMaster.newEmptyAccount();
        const newEoaResponse = await query(`/account/${newEoa.address}`);
        expect(newEoaResponse).toEqual({
            address: newEoa.address.toLowerCase(),
            balances: {},
            sealedNonce: 0,
            verifiedNonce: 0,
            accountType: 'eOA'
        });

        // Check response for the non-empty account.
        const aliceResponse = await query(`/account/${alice.address}`);
        const aliceExpectedBalances: any = {};
        aliceExpectedBalances[micro.utils.ETH_ADDRESS] = await apiBalanceObject(
            micro.utils.ETH_ADDRESS,
            await alice.getBalance()
        );
        aliceExpectedBalances[erc20.l2Address.toLowerCase()] = await apiBalanceObject(
            erc20.l2Address,
            await alice.getBalance(erc20.l2Address),
            erc20.l1Address
        );
        expect(aliceResponse.balances).toEqual(aliceExpectedBalances);
    });

    test('Should test /account endpoint for a contract', async () => {
        // Check response for the empty account.
        const contract = await deployContract(alice, contracts.counter, []);
        const contractResponse = await query(`/account/${contract.address}`);
        expect(contractResponse).toEqual({
            address: contract.address.toLowerCase(),
            balances: {},
            sealedNonce: 0,
            verifiedNonce: 0,
            accountType: 'contract'
        });
    });

    test('Should test /transaction endpoint', async () => {
        const amount = 1;
        const bob = testMaster.newEmptyAccount();
        const txNonce = await alice.getTransactionCount();
        const txHandle = await alice.transfer({ to: bob.address, amount, token: erc20.l2Address });
        const tx = await txHandle.wait();

        const apiTx = await query(`/transaction/${tx.transactionHash}`);
        expect(apiTx).toMatchObject({
            transactionHash: tx.transactionHash,
            nonce: txNonce,
            blockNumber: tx.blockNumber,
            blockHash: tx.blockHash,
            indexInBlock: expect.any(Number),
            status: expect.stringMatching(/included|verified/),
            fee: ethers.utils.hexValue(extractFee(tx as any).feeAfterRefund),
            isL1Originated: false,
            initiatorAddress: alice.address.toLowerCase(),
            receivedAt: expect.stringMatching(DATE_REGEX),
            miniblockTimestamp: expect.any(Number),
            balanceChanges: expect.any(Array),
            erc20Transfers: expect.any(Array),
            data: {
                calldata: txHandle.data,
                contractAddress: erc20.l2Address.toLowerCase(),
                factoryDeps: null,
                value: ethers.utils.hexValue(txHandle.value)
            },
            logs: expect.any(Array),
            transfer: {
                from: alice.address.toLowerCase(),
                to: bob.address.toLowerCase(),
                amount: ethers.utils.hexValue(amount),
                tokenInfo: await erc20TokenInfo(erc20.l2Address, erc20.l1Address)
            }
        });

        if (!testMaster.isFastMode()) {
            // Wait for the block to become verified and check that the corresponding fields are set.
            await waitFor(async () => {
                const verifiedBlock = await query(`/block/${tx.blockNumber}`);
                return verifiedBlock.status == 'verified';
            }, 'Block was not verified');

            const finalizedApiTx = await query(`/transaction/${tx.transactionHash}`);
            expect(finalizedApiTx).toMatchObject({
                ethCommitTxHash: expect.stringMatching(HASH_REGEX),
                ethProveTxHash: expect.stringMatching(HASH_REGEX),
                ethExecuteTxHash: expect.stringMatching(HASH_REGEX),
                l1BatchNumber: expect.any(Number)
            });
        }
    });

    test('Should test /transaction endpoint for L1->L2', async () => {
        if (testMaster.isFastMode()) {
            // This test requires an L1->L2 transaction to be included, which may be time consuming on stage.
            return;
        }

        const amount = 1;
        const txHandle = await alice.deposit({ to: alice.address, amount, token: erc20.l1Address, approveERC20: true });
        const tx = await txHandle.wait();

        const apiTx = await query(`/transaction/${tx.transactionHash}`);
        expect(apiTx).toMatchObject({
            transactionHash: tx.transactionHash,
            blockNumber: tx.blockNumber,
            blockHash: tx.blockHash,
            indexInBlock: expect.any(Number),
            status: expect.stringMatching(/included|verified/),
            fee: ethers.utils.hexValue(tx.gasUsed.mul(tx.effectiveGasPrice)),
            isL1Originated: true,
            initiatorAddress: expect.stringMatching(HEX_VALUE_REGEX),
            receivedAt: expect.stringMatching(DATE_REGEX),
            miniblockTimestamp: expect.any(Number),
            balanceChanges: expect.any(Array),
            erc20Transfers: expect.any(Array),
            data: {
                calldata: expect.stringMatching(HEX_VALUE_REGEX),
                contractAddress: expect.stringMatching(ADDRESS_REGEX),
                factoryDeps: expect.any(Array),
                value: expect.stringMatching(HEX_VALUE_REGEX)
            },
            logs: expect.any(Array)
        });
    });

    test('Should test /transactions endpoint', async () => {
        const amount = 1;
        const bob = testMaster.newEmptyAccount();
        const txNonce = await alice.getNonce();
        const tx = await alice.transfer({ to: bob.address, amount }).then((tx) => tx.wait());

        const response: any = await query('/transactions', {
            blockNumber: tx.blockNumber.toString(),
            limit: '100',
            direction: 'older'
        });
        expect(response).toEqual({
            total: expect.any(Number),
            list: expect.anything()
        });
        expect(response.total).toBeGreaterThanOrEqual(1);

        const apiTx = response.list.find((apiTx: any) => apiTx.transactionHash == tx.transactionHash);
        expect(apiTx).toBeDefined();

        // Ensure the response format based on the performed ETH transfer.
        // After this check we assume that the response format is the same in other responses
        // to avoid being too verbose.
        expect(apiTx).toMatchObject({
            transactionHash: tx.transactionHash,
            nonce: txNonce,
            blockNumber: tx.blockNumber,
            blockHash: tx.blockHash,
            indexInBlock: expect.any(Number),
            status: expect.stringMatching(/included|verified/),
            fee: ethers.utils.hexValue(extractFee(tx as any).feeAfterRefund),
            isL1Originated: false,
            initiatorAddress: alice.address.toLowerCase(),
            receivedAt: expect.stringMatching(DATE_REGEX),
            miniblockTimestamp: expect.any(Number),
            balanceChanges: expect.any(Array),
            erc20Transfers: expect.any(Array),
            data: {
                calldata: '0x',
                contractAddress: bob.address.toLowerCase(),
                factoryDeps: null,
                value: ethers.utils.hexValue(amount)
            },
            transfer: {
                from: alice.address.toLowerCase(),
                to: bob.address.toLowerCase(),
                amount: ethers.utils.hexValue(amount),
                tokenInfo: {
                    address: micro.utils.ETH_ADDRESS,
                    l1Address: micro.utils.ETH_ADDRESS,
                    l2Address: micro.utils.ETH_ADDRESS,
                    symbol: 'ETH',
                    name: 'Ether',
                    decimals: 18,
                    usdPrice: expect.any(String)
                }
            },
            type: tx.type
        });

        // Perform L1 batch-related checks in the long mode only.
        if (!testMaster.isFastMode()) {
            const tx = await waitForNewL1Batch(alice);
            const response: any = await query('/transactions', {
                l1BatchNumber: tx.l1BatchNumber.toString(),
                limit: '100',
                direction: 'older'
            });
            expect(response).toEqual({
                total: expect.any(Number),
                list: expect.anything()
            });
            expect(response.total).toBeGreaterThanOrEqual(1);

            const apiTx = response.list.find((apiTx: any) => apiTx.transactionHash == tx.transactionHash);
            expect(apiTx).toBeDefined();
        }

        // Check other query parameters combinations
        const backwards = await query('/transactions', {
            limit: '1',
            direction: 'older'
        });
        expect(backwards.list.length).toEqual(1);

        const forward = await query('/transactions', {
            limit: '1',
            offset: '1',
            direction: 'newer'
        });
        expect(forward.list.length).toEqual(1);

        const tom = testMaster.newEmptyAccount();
        await alice.transfer({ to: tom.address, amount }).then((tx) => tx.wait());

        // Alice sent at least 2 txs: to Bob and to Tom.
        let accountTxs = await query('/transactions', {
            limit: '2',
            direction: 'older',
            accountAddress: alice.address
        });
        expect(accountTxs.list.length).toEqual(2);
        // Tom received only 1 tx from Alice.
        accountTxs = await query('/transactions', {
            limit: '10',
            direction: 'older',
            accountAddress: tom.address
        });
        expect(accountTxs.list.length).toEqual(1);

        // Invariant: ERC20 tokens are distributed during init, so it must have transactions.
        const contract = await query('/transactions', {
            limit: '1',
            direction: 'older',
            contractAddress: erc20.l2Address
        });
        expect(contract.list.length).toEqual(1);
    });

    test('Should test /contract endpoint', async () => {
        const counterContract = await deployContract(alice, contracts.counter, []);
        const createdInBlockNumber = (
            await alice.provider.getTransactionReceipt(counterContract.deployTransaction.hash)
        ).blockNumber;
        const apiContractInfo = await query(`/contract/${counterContract.address}`);
        expect(apiContractInfo).toEqual({
            address: counterContract.address.toLowerCase(),
            creatorAddress: alice.address.toLowerCase(),
            creatorTxHash: counterContract.deployTransaction.hash,
            createdInBlockNumber,
            totalTransactions: 0,
            bytecode: ethers.utils.hexlify(contracts.counter.bytecode),
            verificationInfo: null,
            balances: {}
        });

        // ERC20 contract is guaranteed to have more than 0 transactions.
        const apiErc20Info = await query(`/contract/${erc20.l2Address}`);
        expect(apiErc20Info.totalTransactions).toBeGreaterThan(0);
    });

    test('Should test /events endpoint', async () => {
        const apiEvents = await query('/events', {
            direction: 'older',
            limit: '100',
            fromBlockNumber: (await alice.provider.getBlockNumber()).toString()
        });
        // Check generic API response structure.
        expect(apiEvents).toEqual({
            list: expect.anything(),
            total: expect.any(Number)
        });
        expect(apiEvents.total).toBeGreaterThan(0);
        expect(apiEvents.list.length).toBeGreaterThan(0);
        expect(apiEvents.list[0]).toMatchObject({
            address: expect.stringMatching(ADDRESS_REGEX),
            blockHash: expect.stringMatching(HASH_REGEX),
            blockNumber: expect.stringMatching(HEX_VALUE_REGEX),
            data: expect.stringMatching(HEX_VALUE_REGEX),
            logIndex: expect.stringMatching(HEX_VALUE_REGEX),
            removed: expect.any(Boolean),
            topics: expect.any(Array),
            transactionHash: expect.stringMatching(HASH_REGEX),
            transactionIndex: expect.stringMatching(HEX_VALUE_REGEX),
            transactionLogIndex: expect.stringMatching(HEX_VALUE_REGEX)
        });

        // Test per-contract filtering.
        const apiErc20Events = await query('/events', {
            direction: 'older',
            limit: '100',
            contractAddress: erc20.l2Address
        });
        for (const apiEvent of apiErc20Events.list) {
            expect(apiEvent.address).toEqual(erc20.l2Address.toLowerCase());
        }
    });

    test('Should test /token endpoint', async () => {
        const apiToken = await query(`/token/${erc20.l2Address}`);
        expect(apiToken).toEqual(await erc20TokenInfo(erc20.l2Address, erc20.l1Address));
    });

    test('should test contract verification', async () => {
        if (process.env.RUN_CONTRACT_VERIFICATION_TEST != 'true') {
            // Contract verification test is not requested to run.
            return;
        }

        const counterContract = await deployContract(alice, contracts.counter, []);
        const constructorArguments = counterContract.interface.encodeDeploy([]);

        const requestBody = {
            contractAddress: counterContract.address,
            contractName: 'contracts/counter/counter.sol:Counter',
            sourceCode: getContractSource('counter/counter.sol'),
            compilerZksolcVersion: ZKSOLC_VERSION,
            compilerSolcVersion: SOLC_VERSION,
            optimizationUsed: true,
            constructorArguments,
            isSystem: true
        };
        let requestId = await query('/contract_verification', undefined, requestBody);

        await expectVerifyRequestToSucceed(requestId, counterContract.address);
    });

    test('should test multi-files contract verification', async () => {
        if (process.env.RUN_CONTRACT_VERIFICATION_TEST != 'true') {
            // Contract verification test is not requested to run.
            return;
        }

        const contractFactory = new micro.ContractFactory(contracts.create.abi, contracts.create.bytecode, alice);
        const contractHandle = await contractFactory.deploy({
            customData: {
                factoryDeps: [contracts.create.factoryDep]
            }
        });
        const importContract = await contractHandle.deployed();
        const standardJsonInput = {
            language: 'Solidity',
            sources: {
                'contracts/create/create.sol': { content: getContractSource('create/create.sol') },
                'contracts/create/Foo.sol': { content: getContractSource('create/Foo.sol') }
            },
            settings: {
                optimizer: { enabled: true },
                isSystem: true
            }
        };

        const constructorArguments = importContract.interface.encodeDeploy([]);

        const requestBody = {
            contractAddress: importContract.address,
            contractName: 'contracts/create/create.sol:Import',
            sourceCode: standardJsonInput,
            codeFormat: 'solidity-standard-json-input',
            compilerZksolcVersion: ZKSOLC_VERSION,
            compilerSolcVersion: SOLC_VERSION,
            optimizationUsed: true,
            constructorArguments
        };
        let requestId = await query('/contract_verification', undefined, requestBody);

        await expectVerifyRequestToSucceed(requestId, importContract.address);
    });

    test('should test yul contract verification', async () => {
        if (process.env.RUN_CONTRACT_VERIFICATION_TEST != 'true') {
            // Contract verification test is not requested to run.
            return;
        }
        const contractPath = `${process.env.MICRO_HOME}/core/tests/ts-integration/contracts/yul/Empty.yul`;
        const sourceCode = fs.readFileSync(contractPath, 'utf8');

        const bytecodePath = `${process.env.MICRO_HOME}/core/tests/ts-integration/contracts/yul/artifacts/Empty.yul/Empty.yul.zbin`;
        const bytecode = fs.readFileSync(bytecodePath);

        const contractFactory = new micro.ContractFactory([], bytecode, alice);
        const deployTx = await contractFactory.deploy();
        const contractAddress = (await deployTx.deployed()).address;

        const requestBody = {
            contractAddress,
            contractName: 'Empty',
            sourceCode,
            codeFormat: 'yul-single-file',
            compilerZksolcVersion: ZKSOLC_VERSION,
            compilerSolcVersion: SOLC_VERSION,
            optimizationUsed: true,
            constructorArguments: '0x',
            isSystem: true
        };
        let requestId = await query('/contract_verification', undefined, requestBody);

        await expectVerifyRequestToSucceed(requestId, contractAddress);
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });

    /**
     * Performs an API call to the Explorer API.
     *
     * @param endpoint API endpoint to call.
     * @param queryParams Parameters for a query string.
     * @param requestBody Request body. If provided, a POST request would be met and body would be encoded to JSON.
     * @returns API response parsed as a JSON.
     */
    async function query(endpoint: string, queryParams?: { [key: string]: string }, requestBody?: any): Promise<any> {
        const url = new URL(endpoint, testMaster.environment().explorerUrl);
        // Iterate through query params and add them to URL.
        if (queryParams) {
            Object.entries(queryParams).forEach(([key, value]) => url.searchParams.set(key, value));
        }

        let init = undefined;
        if (requestBody) {
            init = {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(requestBody)
            };
        }

        let response = await fetch(url, init);
        try {
            return await response.json();
        } catch (e) {
            throw {
                error: 'Could not decode JSON in response',
                status: `${response.status} ${response.statusText}`
            };
        }
    }

    /**
     * Constructs an Explorer API balance object representation
     */
    async function apiBalanceObject(address: string, balance: ethers.BigNumber, l1Address?: string) {
        address = address.toLowerCase();
        // `hexValue` can contain an uneven number of nibbles (unlike `.toHexString()`), which is required for API.
        const hexBalance = ethers.utils.hexValue(balance);
        if (address == micro.utils.ETH_ADDRESS) {
            return {
                balance: hexBalance,
                tokenInfo: {
                    address,
                    decimals: 18,
                    l1Address: address,
                    l2Address: address,
                    name: 'Ether',
                    symbol: 'ETH',
                    usdPrice: expect.any(String)
                }
            };
        }

        return {
            balance: hexBalance,
            tokenInfo: await erc20TokenInfo(address, l1Address)
        };
    }

    /**
     * Constructs an object that represent the token information sent by the Explorer API.
     */
    async function erc20TokenInfo(address: string, l1Address?: string) {
        const erc20 = IERC20MetadataFactory.connect(address, alice);
        return {
            address: address.toLowerCase(),
            decimals: await erc20.decimals(),
            l1Address: l1Address ? l1Address.toLowerCase() : expect.stringMatching(ADDRESS_REGEX),
            l2Address: address.toLowerCase(),
            name: await erc20.name(),
            symbol: await erc20.symbol(),
            usdPrice: expect.any(String)
        };
    }

    /**
     * Runs a provided asynchronous predicate until it returns `true`.
     * If it doesn't happen for a while, fails the test from which it has been called.
     */
    async function waitFor(cond: () => Promise<boolean>, errorMessage: string) {
        const MAX_RETRIES = 15_000;
        let iter = 0;
        while (iter++ < MAX_RETRIES) {
            if (await cond()) {
                return;
            }
            await sleep(alice.provider.pollingInterval);
        }

        expect(null).fail(errorMessage);
    }

    async function expectVerifyRequestToSucceed(requestId: number, contractAddress: string) {
        let retries = 0;
        while (true) {
            if (retries > 100) {
                throw new Error('Too many retries');
            }

            let statusObject = await query(`/contract_verification/${requestId}`);
            if (statusObject.status == 'successful') {
                break;
            } else if (statusObject.status == 'failed') {
                throw new Error(statusObject.error);
            } else {
                retries += 1;
                await sleep(alice.provider.pollingInterval);
            }
        }

        let contractObject = await query(`/contract/${contractAddress}`);
        expect(contractObject.verificationInfo).toBeTruthy();
    }
});

/**
 * Checks that timestamp has some relatively sane value (not too much in the past, and not in the future)
 */
function expectTimestampToBeSane(timestamp: number) {
    const minDate = new Date('01 Jan 2022 00:00:00 UTC').getSeconds();
    const maxDate = Date.now();
    expect(timestamp).toBeGreaterThan(minDate);
    expect(timestamp).toBeLessThanOrEqual(maxDate);
}
