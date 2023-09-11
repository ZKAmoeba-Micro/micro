// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./libraries/TransferHelper.sol";
import "./openzeppelin/utils/math/SafeMath.sol";
import "./interfaces/IFeePool.sol";
import "./interfaces/IDeposit.sol";
import "./interfaces/IProofRewardPool.sol";

import {BOOTLOADER_FORMAL_ADDRESS, DEPOSIT_ADDRESS, PROOF_REWARD_POOL_ADDRESS} from "./Constants.sol";

contract FeePool is IFeePool {
    using SafeMath for uint256;

    modifier onlyBootloader() {
        require(
            msg.sender == BOOTLOADER_FORMAL_ADDRESS,
            "only bootloader can call"
        );
        _;
    }

    modifier onlyProofRewardPool() {
        require(
            msg.sender == PROOF_REWARD_POOL_ADDRESS,
            "only proof reward pool can call"
        );
        _;
    }

    struct SubmitInfo {
        address server;
        uint256 submitCount;
    }
    uint256 totalSubmitCount;

    mapping(address => bool) serverHadSubmit;
    mapping(address => uint256) serverIndex;
    SubmitInfo[] serverAddresses;

    uint64 lastPeriodId;
    uint64 lastPeriodTimestamp;

    function addFee(address _sender) external payable onlyBootloader {
        // require(IDeposit(DEPOSIT_ADDRESS).isNode(_sender) && msg.value > 0);
        require(msg.value > 0);

        if (serverHadSubmit[_sender]) {
            uint256 index = serverIndex[_sender];
            SubmitInfo storage info = serverAddresses[index];
            info.submitCount += 1;
        } else {
            serverHadSubmit[_sender] = true;
            SubmitInfo memory info = SubmitInfo(_sender, 1);
            serverAddresses.push(info);
            serverIndex[_sender] = serverAddresses.length - 1;
        }
        totalSubmitCount += 1;
    }

    function checkAndPayRewards(
        uint64 _timestamp
    ) external onlyProofRewardPool {
        if (totalSubmitCount == 0) {
            return;
        }
        if (_timestamp < lastPeriodTimestamp + 43200) {
            return;
        }
        lastPeriodId++;
        lastPeriodTimestamp = _timestamp;

        uint256 serverBalance = address(this).balance;
        for (uint256 i = serverAddresses.length; i >= 1; i--) {
            SubmitInfo memory info = serverAddresses[i - 1];
            uint256 reward = serverBalance.mul(info.submitCount).div(
                totalSubmitCount
            );
            serverAddresses.pop();
            serverHadSubmit[info.server] = false;
            serverIndex[info.server] = 0;
            TransferHelper.safeTransferFIL(info.server, reward);
        }
        totalSubmitCount = 0;
    }

    function estimateFeeReward() external view returns (FeeReward[] memory) {
        FeeReward[] memory feeRewards = new FeeReward[](serverAddresses.length);
        uint256 serverBalance = address(this).balance;
        for (uint256 i = serverAddresses.length; i >= 1; i--) {
            SubmitInfo memory info = serverAddresses[i - 1];
            uint256 reward = serverBalance.mul(info.submitCount).div(
                totalSubmitCount
            );
            feeRewards[i-1] = FeeReward({server: info.server, reward: reward});
        }
        return feeRewards;
    }
}
