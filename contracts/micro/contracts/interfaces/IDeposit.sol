// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface IDeposit {
    enum Role {
        Normol,
        FrozenNode,
        Node,
        WaitingNode
    }

    struct NodeInfo {
        address nodeAddress;
        uint256 depositAmount;
        Role nodeRole;
    }

    function token() external view returns (address);

    function isNode(address _nodeAddress) external view returns (bool);

    function nodeDepositAmount(address _nodeAddress) external view returns (uint256);

    function getNodeInfo() external view returns (NodeInfo[] memory);

    function getNodeListLength() external view returns (uint256);

    function getAllNodeInfo(uint256 _start, uint256 _end) external view returns (NodeInfo[] memory);

    function getMinDepositAmount() external view returns (uint256);
}
