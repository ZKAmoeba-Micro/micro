// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface IEthToken {
    function balanceOf(uint256) external view returns (uint256);

    function totalSupply() external view returns (uint256);

    function decimals() external pure returns (uint8);
}
