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

    function estimateReward() external view returns (uint256, uint256, uint256);

    function getBlockFee() external view returns (uint256);

    function getState()
        external
        view
        returns (address, uint256, uint256, uint64, uint16, uint64);
}
