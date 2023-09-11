// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface IFeePool {
    struct FeeReward {
        address server;
        uint256 reward;
    }

    function estimateFeeReward() external view returns (FeeReward[] memory);
}
