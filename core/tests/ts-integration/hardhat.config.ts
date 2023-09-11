import '@zkamoeba/hardhat-micro-solc';

export default {
    zksolc: {
        version: '1.3.10',
        compilerSource: 'binary',
        settings: {
            isSystem: true
        }
    },
    networks: {
        hardhat: {
            micro: true
        }
    },
    solidity: {
        version: '0.8.16'
    }
};
