// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

/// @dev All the system contracts introduced by micro have their addresses
/// started from 2^15 in order to avoid collision with Ethereum precompiles.
uint160 constant SYSTEM_CONTRACTS_OFFSET = 0x8000; // 2^15

address constant ETH_TOKEN_ADDRESS = address(SYSTEM_CONTRACTS_OFFSET + 0x0a);

address constant DEPOSIT_ADDRESS = address(SYSTEM_CONTRACTS_OFFSET + 0x100);

address constant FEE_POOL_ADDRESS = payable(address(SYSTEM_CONTRACTS_OFFSET + 0x101));

address constant PROOF_REWARD_POOL_ADDRESS = address(SYSTEM_CONTRACTS_OFFSET + 0x102);
