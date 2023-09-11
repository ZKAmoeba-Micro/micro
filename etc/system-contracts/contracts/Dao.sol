// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "./interfaces/IDao.sol";

contract Dao is IDao {
    fallback() external payable {}

    receive() external payable {}
}
