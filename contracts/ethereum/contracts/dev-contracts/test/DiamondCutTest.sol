// SPDX-License-Identifier: MIT

pragma solidity ^0.8.13;

import "../../micro/libraries/Diamond.sol";
import "../../micro/facets/Getters.sol";

contract DiamondCutTest is GettersFacet {
    function diamondCut(Diamond.DiamondCutData memory _diamondCut) external {
        Diamond.diamondCut(_diamondCut);
    }
}
