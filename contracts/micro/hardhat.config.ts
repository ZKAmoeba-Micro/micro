import '@nomiclabs/hardhat-solpp';
import '@zkamoeba/hardhat-micro-solc';
import 'hardhat-typechain';

// If no network is specified, use the default config
if (!process.env.CHAIN_ETH_NETWORK) {
    require('dotenv').config();
}

export default {
    zksolc: {
        version: '1.3.7',
        compilerSource: 'binary',
        settings: {
            isSystem: true
        }
    },
    solidity: {
        version: '0.8.19'
    },
    networks: {
        hardhat: {
            micro: true
        }
    }
};
