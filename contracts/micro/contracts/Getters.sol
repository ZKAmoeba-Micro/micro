// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "./interfaces/IDeposit.sol";
import "./interfaces/IProofRewardPool.sol";
import "./interfaces/IFeePool.sol";
import "./interfaces/IEthToken.sol";
import "./interfaces/IGetters.sol";
import {ETH_TOKEN_ADDRESS, DEPOSIT_ADDRESS, FEE_POOL_ADDRESS, PROOF_REWARD_POOL_ADDRESS} from "./Constants.sol";

contract Getters is IGetters {
    function isNode(address _nodeAddress) external view returns (bool) {
        return IDeposit(DEPOSIT_ADDRESS).isNode(_nodeAddress);
    }

    function nodeDepositAmount(address _nodeAddress) external view returns (uint256) {
        return IDeposit(DEPOSIT_ADDRESS).nodeDepositAmount(_nodeAddress);
    }

    function getNodeInfo() external view returns (NodeInfo[] memory) {
        return IDeposit(DEPOSIT_ADDRESS).getNodeInfo();
    }

    function getNodeListLength() external view returns (uint256) {
        return IDeposit(DEPOSIT_ADDRESS).getNodeListLength();
    }

    function getAllNodeInfo(uint256 _start, uint256 _end) external view returns (NodeInfo[] memory) {
        return IDeposit(DEPOSIT_ADDRESS).getAllNodeInfo(_start, _end);
    }

    function getMinDepositAmount() external view returns (uint256) {
        return IDeposit(DEPOSIT_ADDRESS).getMinDepositAmount();
    }

    function token() external view returns (address) {
        return IDeposit(DEPOSIT_ADDRESS).token();
    }

    function estimateReward()
        external
        view
        returns (
            uint256,
            uint256,
            uint256
        )
    {
        return IProofRewardPool(PROOF_REWARD_POOL_ADDRESS).estimateReward();
    }

    function getBlockFee() external view returns (uint256) {
        return IProofRewardPool(PROOF_REWARD_POOL_ADDRESS).getBlockFee();
    }

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
        )
    {
        return IProofRewardPool(PROOF_REWARD_POOL_ADDRESS).getState();
    }

    function totalSupply() external view returns (uint256) {
        return IEthToken(ETH_TOKEN_ADDRESS).totalSupply();
    }

    function getStatistiscInfo() external view returns (StatisticsInfo memory) {
        uint256 fileDeposit = IEthToken(ETH_TOKEN_ADDRESS).totalSupply();
        uint256 blockFee = IProofRewardPool(PROOF_REWARD_POOL_ADDRESS).getBlockFee();
        (uint256 commiterReward, uint256 proverReward, uint256 executeReward) = IProofRewardPool(
            PROOF_REWARD_POOL_ADDRESS
        ).estimateReward();
        uint256 accBlockFee = commiterReward + proverReward + executeReward;
        uint256 nodeListLength = IDeposit(DEPOSIT_ADDRESS).getNodeListLength();
        StatisticsInfo memory statisticsInfo = StatisticsInfo({
            fileDeposit: fileDeposit,
            blockFee: blockFee,
            accBlockFee: accBlockFee,
            nodeListLength: nodeListLength
        });
        return statisticsInfo;
    }

    function estimateFeeReward() external view returns (FeeReward[] memory) {
        return IFeePool(FEE_POOL_ADDRESS).estimateFeeReward();
    }
}
