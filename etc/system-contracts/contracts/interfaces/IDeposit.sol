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

    event AddNode(address indexed node);

    event RemoveNode(address indexed node);

    event PayToProver(address indexed node, uint256 indexed amount);

    function token() external view returns(address);

    function deposit(uint256 _amount) external;

    function withdrawApply() external;

    function withdraw() external;

    function penalize(address _nodeAddress, uint256 _blockHeight) external;

    function slashing(address _nodeAddress) external;

    function payToProver(address _miner, uint256 _amount) external;

    function updateMinDepositAmount(uint256 _minAmount) external;

    function updatePakageCycle(uint256 _pakageCycle) external;

    function updateReleaseTimeCycle(uint256 _releaseTimeCycle) external;

    function updatePenalizeRatio(uint16 _penalizeRatio) external;

    function updateConfiscationVoteRatio(
        uint16 _confiscationVoteRatio
    ) external;

    function updateConfiscationToNodePercent(
        uint16 _confiscationToNodePercent
    ) external;

    function updateToken(address _token) external;

    function isNode(address _nodeAddress) external view returns (bool);

    function nodeDepositAmount(
        address _nodeAddress
    ) external view returns (uint256);

    function getNodeInfo() external view returns (NodeInfo[] memory);

    function getNodeListLength() external view returns (uint256);

    function getAllNodeInfo(
        uint256 _start,
        uint256 _end
    ) external view returns (NodeInfo[] memory);

    function getMinDepositAmount() external view returns (uint256);
}
