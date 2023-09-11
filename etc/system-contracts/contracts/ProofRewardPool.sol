// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "./openzeppelin/token/ERC20/IERC20.sol";
import "./openzeppelin/utils/math/Math.sol";
import "./interfaces/IProofRewardPool.sol";
import "./interfaces/IDeposit.sol";
import "./interfaces/IFeePool.sol";
import "./libraries/FixedPointMath.sol";
import {BOOTLOADER_FORMAL_ADDRESS, SYSTEM_CONTEXT_CONTRACT, DEPOSIT_ADDRESS, FEE_POOL_ADDRESS, PROOF_REWARD_POOL_CALLER_ADDRESS} from "./Constants.sol";

contract ProofRewardPool is IProofRewardPool {
    using Math for uint256;

    address public owner;

    address public token;

    /// @dev Target proof time seconds
    uint64 proofTimeTarget;
    /// @dev proofTimeIssued
    uint64 proofTimeIssued;
    /// @dev adjustmentQuotient
    uint16 adjustmentQuotient;
    /// @dev block fee to pay
    uint256 blockFee;
    /// @dev Accumulate fees of committed block that not verified
    uint256 accBlockFees;

    mapping(address => uint256) rewards;

    struct Block {
        address committer;
        address prover;
        address executer;
        uint256 committedAt;
        uint256 provedAt;
        uint256 executedAt;
    }
    mapping(bytes32 => Block) blocks;

    modifier onlyOwner() {
        require(msg.sender == owner, "This method require owner call flag");
        _;
    }

    modifier onlyBootloader() {
        require(msg.sender == BOOTLOADER_FORMAL_ADDRESS);
        _;
    }

    modifier onlyCaller() {
        require(msg.sender == PROOF_REWARD_POOL_CALLER_ADDRESS);
        _;
    }

    function initialize(
        address _owner,
        address _token,
        uint64 _proofTimeTarget,
        uint16 _adjustmentQuotient
    ) public {
        require(_token != address(0), "Invalid _token");
        require(_owner != address(0), "Invalid _owner");

        owner = _owner;
        token = _token;
        proofTimeTarget = _proofTimeTarget;
        adjustmentQuotient = _adjustmentQuotient;
        blockFee = 1e18;
    }

    function estimateReward() public view returns (uint256, uint256, uint256) {
        uint256 reward = accBlockFees;
        uint256 commiterReward = reward / 10;
        uint256 proverReward = (reward * 8) / 10;
        uint256 executeReward = reward - commiterReward - proverReward;
        return (commiterReward, proverReward, executeReward);
    }

    function getBlockFee() public view returns (uint256) {
        return blockFee;
    }

    function getState()
        public
        view
        returns (address, uint256, uint256, uint64, uint16, uint64)
    {
        return (
            address(token),
            blockFee,
            accBlockFees,
            proofTimeTarget,
            adjustmentQuotient,
            proofTimeIssued
        );
    }

    function getBlockState(
        bytes32 _blockHash
    ) public view returns (Block memory) {
        return blocks[_blockHash];
    }

    function setConfig(
        uint64 _proofTimeTarget,
        uint16 _adjustmentQuotient
    ) public onlyOwner {
        if (_proofTimeTarget != type(uint64).max) {
            proofTimeTarget = _proofTimeTarget;
        }
        if (_adjustmentQuotient != type(uint16).max) {
            adjustmentQuotient = _adjustmentQuotient;
        }
    }

    function onCreateBlock(address _miner) public onlyBootloader {
        // pay token
        if (token == address(0)) {
            return;
        }
        IDeposit(DEPOSIT_ADDRESS).payToProver(_miner, blockFee);
        accBlockFees += blockFee;
    }

    function onCommitBlock(
        uint256 _blockNumber,
        bytes32 _blockHash,
        address _sender
    ) public onlyCaller {
        if (token == address(0)) {
            return;
        }

        if (SYSTEM_CONTEXT_CONTRACT.blockHash(_blockNumber) != _blockHash) {
            return;
        }

        Block storage b = blocks[_blockHash];

        if (b.committer != address(0)) {
            return;
        }

        blocks[_blockHash].committer = _sender;
        blocks[_blockHash].committedAt = block.timestamp;

        IFeePool(FEE_POOL_ADDRESS).checkAndPayRewards(uint64(block.timestamp));
    }

    function onProveBlock(
        uint256 _blockNumber,
        bytes32 _blockHash,
        address _sender
    ) public onlyCaller {
        if (token == address(0)) {
            return;
        }

        if (SYSTEM_CONTEXT_CONTRACT.blockHash(_blockNumber) != _blockHash) {
            return;
        }

        Block storage b = blocks[_blockHash];

        if (b.committer == address(0) || b.prover != address(0)) {
            return;
        }

        b.prover = _sender;
        b.provedAt = block.timestamp;
    }

    function onExecuteBlock(
        uint256 _blockNumber,
        bytes32 _blockHash,
        address _sender
    ) public onlyCaller {
        if (token == address(0)) {
            return;
        }

        if (SYSTEM_CONTEXT_CONTRACT.blockHash(_blockNumber) != _blockHash) {
            return;
        }

        Block storage b = blocks[_blockHash];

        if (
            b.committer == address(0) ||
            b.prover == address(0) ||
            b.executer != address(0)
        ) {
            return;
        }

        uint64 proofTime = uint64(b.provedAt - b.committedAt);

        (proofTimeIssued, blockFee) = getNewBlockFeeAndProofTimeIssued(
            proofTime
        );

        // split reward
        (
            uint256 commiterReward,
            uint256 proverReward,
            uint256 executeReward
        ) = estimateReward();

        accBlockFees = 0;

        b.executer = _sender;
        b.executedAt = block.timestamp;

        rewards[b.committer] += commiterReward;
        rewards[b.prover] += proverReward;
        rewards[b.executer] += executeReward;
    }

    function withdrawReward() public {
        if (token == address(0)) {
            return;
        }
        uint256 amount = rewards[msg.sender];
        if (amount > 0) {
            rewards[msg.sender] = 0;
            require(
                IERC20(token).transfer(msg.sender, amount),
                "transfer failed"
            );
        }
    }

    /// @dev Calculate the newProofTimeIssued and blockFee
    /// @param proofTime The actual proof time
    /// @return newProofTimeIssued Accumulated proof time
    /// @return newBlockFee New block fee
    function getNewBlockFeeAndProofTimeIssued(
        uint64 proofTime
    ) internal view returns (uint64 newProofTimeIssued, uint256 newBlockFee) {
        newProofTimeIssued = (proofTimeIssued > proofTimeTarget)
            ? proofTimeIssued - proofTimeTarget
            : uint64(0);
        newProofTimeIssued += proofTime;

        uint256 x = (newProofTimeIssued * FixedPointMath.SCALING_FACTOR_1E18) /
            (proofTimeTarget * adjustmentQuotient);

        if (FixedPointMath.MAX_EXP_INPUT < x) {
            x = FixedPointMath.MAX_EXP_INPUT;
        }

        uint256 result = (uint256(FixedPointMath.exp(int256(x))) /
            FixedPointMath.SCALING_FACTOR_1E18) /
            (proofTimeTarget * adjustmentQuotient);

        newBlockFee = uint256(result.min(type(uint256).max));

        // Keep it within 0.1 and 10 ZKAT and not allow proofTimeIssued accumulated
        // so that fee could recover quicker when waiting is applied from provers.
        if (newBlockFee < 1e17) {
            newProofTimeIssued -= proofTime;
            newBlockFee = 1e17;
        } else if (newBlockFee > 1e19) {
            newProofTimeIssued -= proofTime;
            newBlockFee = 1e19;
        }
    }
}
