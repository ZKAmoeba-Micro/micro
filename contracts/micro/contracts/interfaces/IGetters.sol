// SPDX-License-Identifier: MIT

pragma solidity ^0.8.13;

import "./IDeposit.sol";
import "./IFeePool.sol";

interface IGetters is IDeposit, IFeePool {
    struct StatisticsInfo {
        uint256 fileDeposit;
        uint256 blockFee;
        uint256 accBlockFee;
        uint256 nodeListLength;
    }

    function getStatistiscInfo() external view returns (StatisticsInfo memory);
}
