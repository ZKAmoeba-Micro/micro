// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface IFeePool {
    struct FeeReward {
        address server;
        uint256 reward;
    }

    function addFee(address _sender) external payable;

    function checkAndPayRewards(uint64 _timestamp) external;

    function estimateFeeReward() external view returns (FeeReward[] memory);
}
