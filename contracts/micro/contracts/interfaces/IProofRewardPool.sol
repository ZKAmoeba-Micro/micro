// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface IProofRewardPool {
    function estimateReward()
        external
        view
        returns (
            uint256,
            uint256,
            uint256
        );

    function getBlockFee() external view returns (uint256);

    function getState()
        external
        view
        returns (
            address,
            uint256,
            uint256,
            uint64,
            uint16,
            uint64
        );
}
