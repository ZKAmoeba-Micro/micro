// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "./openzeppelin/token/ERC20/IERC20.sol";
import "./openzeppelin/utils/Address.sol";
import "./openzeppelin/utils/math/SafeMath.sol";
import "./interfaces/IDeposit.sol";
import "./libraries/TransferHelper.sol";

import {BOOTLOADER_FORMAL_ADDRESS, PROOF_REWARD_POOL_ADDRESS, DAO_ADDRESS} from "./Constants.sol";

contract Deposit is IDeposit {
    using SafeMath for uint256;

    address public owner;

    /// @dev deposit token to be node
    address public token;
    /// @dev can pakage block after cycle,uint: block height
    uint256 pakageCycle;
    /// @dev min deposit amount to be node
    uint256 minDepositAmount;
    /// @dev cancel the minimum release cycle of a node,uint: seconds
    uint256 releaseTimeCycle;
    /// @dev min vote ratio for penalize
    uint16 penalizeRatio;
    /// @dev min vote ratio for confiscation
    uint16 confiscationVoteRatio;
    /// @dev confiscationTo amount awarded to node reward ratio
    uint16 confiscationToNodePercent;
    /// @dev base denominator
    uint16 constant denominator = 1000;

    address[] nodeList;

    mapping(address => uint256) nodeIndex;

    mapping(address => Role) addressRole;
    /// @dev can pacake block min block height
    mapping(address => uint256) pakageHeight;

    mapping(address => bool) public override isNode;

    mapping(address => uint256) public override nodeDepositAmount;
    /// @dev time allowed for withdraw
    mapping(address => uint256) releaseTime;

    /// @dev black list
    mapping(address => bool) blackList;

    modifier onlyOwner() {
        require(msg.sender == owner, "This method require owner call flag");
        _;
    }

    function initialize(
        address _owner,
        address _token,
        uint256 _pakageCycle,
        uint256 _minDepositAmount,
        uint256 _releaseTimeCycle,
        uint16 _penalizeRatio,
        uint16 _confiscationVoteRatio,
        uint16 _confiscationToNodePercent
    ) public {
        require(_owner != address(0), "Invalid params");
        require(_token != address(0), "Invalid params");
        require(_pakageCycle >= 0, "Invalid params");
        require(_minDepositAmount > 0, "Invalid params");
        require(_releaseTimeCycle >= 0, "Invalid params");
        require(
            _penalizeRatio > 0 && _penalizeRatio <= denominator,
            "Invalid params"
        );
        require(
            _confiscationVoteRatio > 0 && _confiscationVoteRatio <= denominator,
            "Invalid params"
        );
        require(
            _confiscationToNodePercent >= 0 &&
                _confiscationToNodePercent <= denominator,
            "Invalid params"
        );
        owner = _owner;
        token = _token;
        pakageCycle = _pakageCycle;
        minDepositAmount = _minDepositAmount;
        releaseTimeCycle = _releaseTimeCycle;
        penalizeRatio = _penalizeRatio;
        confiscationVoteRatio = _confiscationVoteRatio;
        confiscationToNodePercent = _confiscationToNodePercent;
    }

    function deposit(uint256 _amount) external override {
        require(token != address(0), "not initted");
        require(!Address.isContract(msg.sender), "only eoa");
        require(!blackList[msg.sender], "address in black list");
        require(
            IERC20(token).balanceOf(msg.sender) >= _amount,
            "Insufficient account balance"
        );

        uint256 depositAmount = nodeDepositAmount[msg.sender];
        require(depositAmount.add(_amount) >= minDepositAmount);

        TransferHelper.safeTransferFrom(
            token,
            msg.sender,
            address(this),
            _amount
        );

        if (addressRole[msg.sender] != Role.Node) {
            addressRole[msg.sender] = Role.Node;
        }
        if (!isNode[msg.sender]) {
            uint256 nodeListLength = nodeList.length;
            isNode[msg.sender] = true;
            nodeList.push(msg.sender);
            nodeIndex[msg.sender] = nodeListLength;
            pakageHeight[msg.sender] = block.number + pakageCycle;
            emit AddNode(msg.sender);
        }
        nodeDepositAmount[msg.sender] += _amount;
    }

    function withdrawApply() external override {
        require(isNode[msg.sender], "only node can apply");
        releaseTime[msg.sender] = block.timestamp + releaseTimeCycle;
        addressRole[msg.sender] = Role.FrozenNode;
    }

    function withdraw() external override {
        require(addressRole[msg.sender] == Role.FrozenNode, "apply first");
        require(block.timestamp >= releaseTime[msg.sender], "not yet time");
        uint256 depositAmount = nodeDepositAmount[msg.sender];

        removeFromNodeList(msg.sender);
        TransferHelper.safeTransfer(token, msg.sender, depositAmount);
    }

    function penalize(address _nodeAddress, uint256 _blockHeight) external {
        //todo
    }

    function slashing(address _nodeAddress) external {
        //todo
    }

    function getNodeInfo() external view override returns (NodeInfo[] memory) {
        NodeInfo[] memory allInfo;
        uint256 nodeListLength = nodeList.length;
        if (nodeListLength == 0) {
            return allInfo;
        }
        NodeInfo[] memory tmp = new NodeInfo[](nodeListLength);
        uint256 length = 0;
        for (uint256 i = 0; i < nodeListLength; i++) {
            address nodeAddress = nodeList[i];
            uint256 depositAmount = nodeDepositAmount[nodeAddress];
            if (depositAmount < minDepositAmount) {
                continue;
            }
            Role role = addressRole[nodeAddress];
            if (role != Role.Node) {
                continue;
            }
            if (pakageHeight[nodeAddress] > block.number) {
                continue;
            }
            tmp[length] = NodeInfo({
                nodeAddress: nodeAddress,
                depositAmount: depositAmount,
                nodeRole: role
            });
            length++;
        }
        allInfo = new NodeInfo[](length);
        for (uint256 i = 0; i < length; i++) {
            allInfo[i] = tmp[i];
        }
        return allInfo;
    }

    function getNodeListLength() external view override returns (uint256) {
        return nodeList.length;
    }

    function getAllNodeInfo(
        uint256 _start,
        uint256 _end
    ) external view override returns (NodeInfo[] memory) {
        uint256 nodeListLength = nodeList.length;
        require(
            _end >= _start &&
                _start >= 0 &&
                _start < nodeListLength &&
                _end < nodeListLength,
            "Invalid params"
        );

        NodeInfo[] memory allInfo;

        if (nodeListLength == 0) {
            return allInfo;
        }

        address[] memory addressList = nodeList;
        allInfo = new NodeInfo[](_end - _start + 1);
        for (uint256 i = _start; i <= _end; i++) {
            address nodeAddress = addressList[i];
            uint256 depositAmount = nodeDepositAmount[nodeAddress];
            Role role = addressRole[nodeAddress];
            allInfo[i] = NodeInfo({
                nodeAddress: nodeAddress,
                depositAmount: depositAmount,
                nodeRole: role
            });
        }
        return allInfo;
    }

    function getMinDepositAmount() external view override returns (uint256) {
        return minDepositAmount;
    }

    function removeFromNodeList(address _node) private {
        require(isNode[_node], "only node can be removed");
        uint256 index = nodeIndex[_node];
        uint256 nodeListLength = nodeList.length;

        // If the node is not at the last position in the node list,
        // then swap the current node and the last position of the node
        if (index + 1 < nodeListLength) {
            address lastNode = nodeList[nodeListLength - 1];
            nodeIndex[lastNode] = index;
            nodeList[index] = lastNode;
        }

        nodeList.pop();
        nodeIndex[_node] = 0;
        isNode[_node] = false;
        addressRole[_node] = Role.Normol;
        nodeDepositAmount[_node] = 0;
        emit RemoveNode(_node);
    }

    function payToProver(address _miner, uint256 _amount) external {
        require(msg.sender == PROOF_REWARD_POOL_ADDRESS, "Invalid sender");
        if (nodeList.length == 0) {
            return;
        }
        require(isNode[_miner], "only noder");
        uint256 depositAmount = nodeDepositAmount[_miner];
        require(_amount <= depositAmount, "Insufficient deposit amount");

        depositAmount = depositAmount.sub(_amount);
        if (depositAmount < minDepositAmount) {
            addressRole[_miner] = Role.WaitingNode;
        }
        nodeDepositAmount[_miner] = depositAmount;
        TransferHelper.safeTransfer(token, PROOF_REWARD_POOL_ADDRESS, _amount);
        emit PayToProver(_miner, _amount);
    }

    function updateMinDepositAmount(uint256 _minAmount) external onlyOwner {
        require(_minAmount > 0, "Invalid params");
        minDepositAmount = _minAmount;
    }

    function updatePakageCycle(uint256 _pakageCycle) external onlyOwner {
        require(
            _pakageCycle >= 0 && _pakageCycle <= denominator,
            "Invalid params"
        );
        pakageCycle = _pakageCycle;
    }

    function updateReleaseTimeCycle(
        uint256 _releaseTimeCycle
    ) external onlyOwner {
        require(_releaseTimeCycle >= 0, "Invalid params");
        releaseTimeCycle = _releaseTimeCycle;
    }

    function updatePenalizeRatio(uint16 _penalizeRatio) external onlyOwner {
        require(
            _penalizeRatio > 0 && _penalizeRatio <= denominator,
            "Invalid params"
        );
        penalizeRatio = _penalizeRatio;
    }

    function updateConfiscationVoteRatio(
        uint16 _confiscationVoteRatio
    ) external onlyOwner {
        require(
            _confiscationVoteRatio > 0 && _confiscationVoteRatio <= denominator,
            "Invalid params"
        );
        confiscationVoteRatio = _confiscationVoteRatio;
    }

    function updateConfiscationToNodePercent(
        uint16 _confiscationToNodePercent
    ) external onlyOwner {
        require(
            _confiscationToNodePercent >= 0 &&
                _confiscationToNodePercent <= denominator,
            "Invalid params"
        );
        confiscationToNodePercent = _confiscationToNodePercent;
    }

    function updateToken(address _token) external onlyOwner {
        require(token != address(0), "not initted");
        require(_token != address(0) && _token != token, "Invalid params");
        token = _token;
        // clean all data
        uint256 nodeListLength = nodeList.length;
        if (nodeListLength > 0) {
            for (uint i = nodeListLength; i >= 1; i--) {
                address nodeAddress = nodeList[i - 1];
                removeFromNodeList(nodeAddress);
            }
        }
    }
}
