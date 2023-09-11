// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "../../bridge/L1ERC20Bridge.sol";

/// @author Matter Labs
contract L1ERC20BridgeTest is L1ERC20Bridge {
    constructor(IMicro _micro, IAllowList _allowList) L1ERC20Bridge(_micro, _allowList) {}

    function getAllowList() public view returns (IAllowList) {
        return allowList;
    }

    function getMicroMailbox() public view returns (IMailbox) {
        return micro;
    }
}
