// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface IProofRewardPool {
    function onCommitBlock(
        uint256 _blockNumber,
        bytes32 _blockHash,
        address _sender
    ) external;

    function onProveBlock(
        uint256 _blockNumber,
        bytes32 _blockHash,
        address _sender
    ) external;

    function onExecuteBlock(
        uint256 _blockNumber,
        bytes32 _blockHash,
        address _sender
    ) external;
}
