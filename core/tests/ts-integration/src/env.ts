import * as path from 'path';
import * as fs from 'fs';
import * as ethers from 'ethers';
import * as micro from 'micro-web3';
import { getTokens } from 'reading-tool';
import { TestEnvironment } from './types';
import { Reporter } from './reporter';

/**
 * Attempts to connect to server.
 * This function returns once connection can be established, or throws an exception in case of timeout.
 *
 * This function is expected to be called *before* loading an environment via `loadTestEnvironment`,
 * because the latter expects server to be running and may throw otherwise.
 */
export async function waitForServer() {
    const reporter = new Reporter();
    // Server startup may take a lot of time on the staging.
    const attemptIntervalMs = 1000;
    const maxAttempts = 20 * 60; // 20 minutes

    const l2NodeUrl = ensureVariable(
        process.env.MICRO_WEB3_API_URL || process.env.API_WEB3_JSON_RPC_HTTP_URL,
        'L2 node URL'
    );
    const l2Provider = new micro.Provider(l2NodeUrl);

    reporter.startAction('Connecting to server');
    let ready = false;
    for (let i = 0; i < maxAttempts; ++i) {
        try {
            await l2Provider.getNetwork(); // Will throw if the server is not ready yet.
            ready = true;
            reporter.finishAction();
            return;
        } catch (e) {
            reporter.message(`Attempt #${i + 1} to check the server readiness failed`);
            await micro.utils.sleep(attemptIntervalMs);
        }
    }

    if (!ready) {
        throw new Error('Failed to wait for the server to start');
    }
}

/**
 * Loads the test environment from the env variables.
 */
export async function loadTestEnvironment(): Promise<TestEnvironment> {
    const network = process.env.CHAIN_ETH_NETWORK || 'localhost';

    let mainWalletPK;
    if (network == 'localhost') {
        const testConfigPath = path.join(process.env.MICRO_HOME!, `etc/test_config/constant`);
        const ethTestConfig = JSON.parse(fs.readFileSync(`${testConfigPath}/eth.json`, { encoding: 'utf-8' }));
        mainWalletPK = ethers.Wallet.fromMnemonic(ethTestConfig.test_mnemonic as string, "m/44'/60'/0'/0/0").privateKey;
    } else {
        mainWalletPK = ensureVariable(process.env.MASTER_WALLET_PK, 'Main wallet private key');
    }

    const l2NodeUrl = ensureVariable(
        process.env.MICRO_WEB3_API_URL || process.env.API_WEB3_JSON_RPC_HTTP_URL,
        'L2 node URL'
    );
    const l1NodeUrl = ensureVariable(process.env.L1_RPC_ADDRESS || process.env.ETH_CLIENT_WEB3_URL, 'L1 node URL');
    const wsL2NodeUrl = ensureVariable(
        process.env.MICRO_WEB3_WS_API_URL || process.env.API_WEB3_JSON_RPC_WS_URL,
        'WS L2 node URL'
    );
    const explorerUrl = ensureVariable(process.env.API_EXPLORER_URL, 'Explorer API');

    const tokens = getTokens(process.env.CHAIN_ETH_NETWORK || 'localhost');
    // wBTC is chosen because it has decimals different from ETH (8 instead of 18).
    // Using this token will help us to detect decimals-related errors.
    const wBTC = tokens.find((token: { symbol: string }) => token.symbol == 'wBTC')!;

    // `waitForServer` is expected to be executed. Otherwise this call may throw.
    const wBTCl2Address = await new micro.Wallet(
        mainWalletPK,
        new micro.Provider(l2NodeUrl),
        ethers.getDefaultProvider(l1NodeUrl)
    ).l2TokenAddress(wBTC.address);

    return {
        network,
        mainWalletPK,
        l2NodeUrl,
        l1NodeUrl,
        wsL2NodeUrl,
        explorerUrl,
        erc20Token: {
            name: wBTC.name,
            symbol: wBTC.symbol,
            decimals: wBTC.decimals,
            l1Address: wBTC.address,
            l2Address: wBTCl2Address
        }
    };
}

/**
 * Checks that variable is not `undefined`, throws an error otherwise.
 */
function ensureVariable(value: string | undefined, variableName: string): string {
    if (!value) {
        throw new Error(`${variableName} is not defined in the env`);
    }
    return value;
}
