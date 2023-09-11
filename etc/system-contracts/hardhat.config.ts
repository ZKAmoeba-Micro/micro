import '@nomiclabs/hardhat-solpp';
import 'hardhat-typechain';
import '@nomiclabs/hardhat-ethers';
import '@zkamoeba/hardhat-micro-solc';

const systemConfig = require('./SystemConfig.json');

export default {
    zksolc: {
        version: '1.3.7',
        compilerSource: 'binary',
        settings: {
            isSystem: true
        }
    },
    solidity: {
        version: '0.8.17'
    },
    solpp: {
        defs: (() => {
            return {
                ECRECOVER_COST_GAS: systemConfig.ECRECOVER_COST_GAS,
                KECCAK_ROUND_COST_GAS: systemConfig.KECCAK_ROUND_COST_GAS,
                SHA256_ROUND_COST_GAS: systemConfig.SHA256_ROUND_COST_GAS
            }
        })()
    },
    defaultNetwork: "microTestnet",
    networks: {
        hardhat: {
            micro: true
        },
        microTestnet: {
            url: 'http://127.0.0.1:3050',
            fileNetwork: 'http://127.0.0.1:1234/rpc/v1',
            micro: true,
        },
    }
};
