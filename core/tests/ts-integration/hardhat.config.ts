import '@zkamoeba/hardhat-micro-solc';
import '@nomiclabs/hardhat-vyper';
import '@zkamoeba/hardhat-micro-vyper';

export default {
    zksolc: {
        version: '1.3.18',
        compilerSource: 'binary',
        settings: {
            isSystem: true
        }
    },
    zkvyper: {
        version: '1.3.13',
        compilerSource: 'binary'
    },
    networks: {
        hardhat: {
            micro: true
        }
    },
    solidity: {
        version: '0.8.23'
    },
    vyper: {
        version: '0.3.10'
    }
};
