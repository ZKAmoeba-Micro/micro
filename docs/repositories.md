# Repositories

## zkAmoeba

### Core components

| Internal repository                                         | Public repository                                                     | Description                                               |
| ----------------------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------- |
| [micro-2-dev](https://github.com/ZKAmoeba-Micro/micro-2-dev) | [micro-era](https://github.com/ZKAmoeba-Micro/micro-era)               | zk server logic, including the APIs and database accesses |
| -                                                           | [micro-wallet-vue](https://github.com/ZKAmoeba-Micro/micro-wallet-vue) | Wallet frontend                                           |

### Contracts

| Public repository                                                           | Description                                                                           |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| [micro-contracts](https://github.com/ZKAmoeba-Micro/micro-contracts)               | L1 & L2 contracts, that are used to manage bridges and communication between L1 & L2. |
| [micro-system-contracts](https://github.com/ZKAmoeba-Micro/micro-system-contracts) | Privileged contracts that are running on L2 (like Bootloader oc ContractDeployer)     |
| [v2-testnet-contracts](https://github.com/ZKAmoeba-Micro/v2-testnet-contracts) |                                                                                       |

### Compiler

| Internal repository                                                           | Public repository                                                                     | Description                                                         |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| [compiler-tester](https://github.com/ZKAmoeba-Micro/compiler-tester)             | [micro-compiler-tester](https://github.com/ZKAmoeba-Micro/micro-compiler-tester)             | Integration testing framework for running executable tests on zkEVM |
| [compiler-tests](https://github.com/ZKAmoeba-Micro/compiler-tests)               | [micro-compiler-tests](https://github.com/ZKAmoeba-Micro/micro-compiler-tests)               | Collection of executable tests for zkEVM                            |
| [compiler-llvm](https://github.com/ZKAmoeba-Micro/compiler-llvm)                 | [micro-compiler-llvm](https://github.com/ZKAmoeba-Micro/compiler-llvm)                     | zkEVM fork of the LLVM framework                                    |
| [compiler-solidity](https://github.com/ZKAmoeba-Micro/compiler-solidity)         | [micro-compiler-solidity](https://github.com/ZKAmoeba-Micro/micro-compiler-solidity)         | Solidity Yul/EVMLA compiler front end                               |
| [compiler-vyper](https://github.com/ZKAmoeba-Micro/compiler-vyper)               | [micro-compiler-vyper](https://github.com/ZKAmoeba-Micro/micro-compiler-vyper)               | Vyper LLL compiler front end                                        |
| [compiler-llvm-context](https://github.com/ZKAmoeba-Micro/compiler-llvm-context) | [micro-compiler-llvm-context](https://github.com/ZKAmoeba-Micro/micro-compiler-llvm-context) | LLVM IR generator logic shared by multiple front ends               |
| [compiler-common](https://github.com/ZKAmoeba-Micro/compiler-common)             | [micro-compiler-common](https://github.com/ZKAmoeba-Micro/micro-compiler-common)             | Common compiler constants                                           |
|                                                                               | [micro-compiler-llvm-builder](https://github.com/ZKAmoeba-Micro/micro-compiler-llvm-builder) | Tool for building our fork of the LLVM framework                    |

### zkEVM

| Internal repository                                                     | Public repository                                                               | Description                                                                                                         |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| [zkevm_opcode_defs](https://github.com/ZKAmoeba-Micro/zkevm_opcode_defs)   | [micro-zkevm_opcode_defs](https://github.com/ZKAmoeba-Micro/micro-zkevm_opcode_defs)   | Opcode definitions for zkEVM - main dependency for many other repos                                                 |
| [zk_evm](https://github.com/ZKAmoeba-Micro/zk_evm)                         | [micro-zk_evm](https://github.com/ZKAmoeba-Micro/micro-zk_evm)                         | EVM implementation in pure rust, without circuits                                                                   |
| [sync_vm](https://github.com/ZKAmoeba-Micro/sync_evm)                      | [micro-sync_vm](https://github.com/ZKAmoeba-Micro/micro-sync_vm)                       | EVM implementation using circuits                                                                                   |
| [zkEVM-assembly](https://github.com/ZKAmoeba-Micro/zkEVM-assembly)         | [micro-zkEVM-assembly](https://github.com/ZKAmoeba-Micro/micro-zkEVM-assembly)         | Code for parsing zkEVM assembly                                                                                     |
| [zkevm_test_harness](https://github.com/ZKAmoeba-Micro/zkevm_test_harness) | [micro-zkevm_test_harness](https://github.com/ZKAmoeba-Micro/micro-zkevm_test_harness) | Tests that compare the two implementation of the zkEVM - the non-circuit one (zk_evm) and the circuit one (sync_vm) |
| [circuit_testing](https://github.com/ZKAmoeba-Micro/circuit_testing)       | [micro-cicruit_testing](https://github.com/ZKAmoeba-Micro/micro-circuit_testing)       | ??                                                                                                                  |
| [heavy-ops-service](https://github.com/ZKAmoeba-Micro/heavy-ops-service)   | [micro-heavy-ops-service](https://github.com/ZKAmoeba-Micro/micro-heavy-ops-service)   | Main circuit prover, that requires GPU to run.                                                                      |
| [bellman-cuda](https://github.com/ZKAmoeba-Micro/bellman-cuda)             | [micro-bellman-cuda](https://github.com/ZKAmoeba-Micro/micro-bellman-cuda)             | Cuda implementations for cryptographic functions used by the prover                                                 |
| [zkevm_tester](https://github.com/ZKAmoeba-Micro/zkevm_tester)             | [micro-zkevm_tester](https://github.com/ZKAmoeba-Micro/micro-zkevm_tester)             | Assembly runner for zkEVM testing                                                                                   |

### Tools & contract developers

| Public repository                                               | Description                                                                   |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [local-setup](https://github.com/ZKAmoeba-Micro/local-setup)       | Docker-based zk server (together with L1), that can be used for local testing |
| [zksolc-bin](https://github.com/ZKAmoeba-Micro/zksolc-bin)         | repository with solc compiler binaries                                        |
| [zkvyper-bin](https://github.com/ZKAmoeba-Micro/zkvyper-bin)       | repository with vyper compiler binaries                                       |
| [micro-cli](<(https://github.com/ZKAmoeba-Micro/micro-cli)>)     | Command line tool to interact with micro                                     |
| [hardhat-micro](https://github.com/ZKAmoeba-Micro/hardhat-micro) | Plugins for hardhat                                                           |

### Examples & documentation

| Public repository                                                                     | Description                                                                                            |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [micro-web-micro-docs](https://github.com/ZKAmoeba-Micro/micro-web-micro-docs)             | Public documentation, API descriptions etc. Source code for [public docs](https://era.micro.io/docs/) |
| [micro-tutorial-examples](https://github.com/ZKAmoeba-Micro/micro-tutorial-examples)         | List of tutorials                                                                                      |
| [custom-paymaster-tutorial](https://github.com/ZKAmoeba-Micro/custom-paymaster-tutorial) | ??                                                                                                     |
| [daily-spendlimit-tutorial](https://github.com/ZKAmoeba-Micro/daily-spendlimit-tutorial) | ??                                                                                                     |
| [custom-aa-tutorial](https://github.com/ZKAmoeba-Micro/custom-aa-tutorial)               | Tutorial for Account Abstraction                                                                       |
| [micro-hardhat-with-plugins](https://github.com/ZKAmoeba-Micro/micro-hardhat-with-plugins)   | ??                                                                                                     |
| [micro-hardhat-template](https://github.com/ZKAmoeba-Micro/micro-hardhat-template)     | ??                                                                                                     |

## micro Lite (v1)

| Internal repository                                     | Public repository                                                           | Description                        |
| ------------------------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------- |
| [micro-dev](https://github.com/ZKAmoeba-Micro/micro-dev) | [micro](https://github.com/ZKAmoeba-Micro/micro)                             | micro Lite/v1 implementation      |
|                                                         | [micro-docs](https://github.com/ZKAmoeba-Micro/micro-docs)                   | Public documentation for micro v1 |
|                                                         | [micro-dapp-checkout](https://github.com/ZKAmoeba-Micro/micro-dapp-checkout) | ??                                 |
