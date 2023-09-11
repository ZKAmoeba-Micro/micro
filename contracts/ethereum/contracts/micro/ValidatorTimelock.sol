// SPDX-License-Identifier: MIT

pragma solidity ^0.8.13;

import "@openzeppelin/contracts/access/Ownable2Step.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";
import "./interfaces/IExecutor.sol";
import "./interfaces/IMicro.sol";
import "../common/L2ContractAddresses.sol";
import "../common/interfaces/IProofRewardPool.sol";
import "./Config.sol";

/// @author Matter Labs
/// @notice Intermediate smart contract between the validator EOA account and the micro smart contract.
/// @dev The primary purpose of this contract is to provide a trustless means of delaying block execution without
/// modifying the main micro contract. As such, even if this contract is compromised, it will not impact the main contract.
/// @dev micro actively monitors the chain activity and reacts to any suspicious activity by freezing the chain.
/// This allows time for investigation and mitigation before resuming normal operations.
/// @dev The contract overloads all of the 4 methods, that are used in state transition. When the block is committed, the
/// timestamp is stored for it. Later, when the owner calls the block execution, the contract checks that block
/// was committed not earlier than X time ago.
contract ValidatorTimelock is IExecutor, Ownable2Step {
    using SafeMath for uint256;
    /// @notice The delay between committing and executing blocks is changed.
    event NewExecutionDelay(uint256 _newExecutionDelay);

    /// @notice The validator address is changed.
    event NewValidator(address _oldValidator, address _newValidator);

    /// @dev The main micro smart contract.
    address public immutable microContract;

    /// @dev The mapping of L2 block number => timestamp when it was commited.
    mapping(uint256 => uint256) public committedBlockTimestamp;

    /// @dev The address that can commit/revert/validate/execute blocks.
    address public validator;

    /// @dev The delay between committing and executing blocks.
    uint256 public executionDelay;

    constructor(
        address _initialOwner,
        address _microContract,
        uint256 _executionDelay,
        address _validator
    ) {
        _transferOwnership(_initialOwner);
        microContract = _microContract;
        executionDelay = _executionDelay;
        validator = _validator;
    }

    /// @dev Set new validator address.
    function setValidator(address _newValidator) external onlyOwner {
        address oldValidator = validator;
        validator = _newValidator;
        emit NewValidator(oldValidator, _newValidator);
    }

    /// @dev Set the delay between committing and executing blocks.
    function setExecutionDelay(uint256 _executionDelay) external onlyOwner {
        executionDelay = _executionDelay;
        emit NewExecutionDelay(_executionDelay);
    }

    /// @notice Checks if the caller is a validator.
    modifier onlyValidator() {
        require(msg.sender == validator, "8h");
        _;
    }

    /// @dev Records the timestamp for all provided committed blocks and make
    /// a call to the micro contract with the same calldata.
    function commitBlocks(StoredBlockInfo calldata, CommitBlockInfo[] calldata _newBlocksData)
        external
        payable
        onlyValidator
    {
        require(msg.value > 0, "commitBlocks msg.value is zero!");
        (uint256 fee, uint256 lastFee) = _splitFee(msg.value, _newBlocksData.length);

        for (uint256 i = 0; i < _newBlocksData.length; ++i) {
            committedBlockTimestamp[_newBlocksData[i].blockNumber] = block.timestamp;

            bytes memory l2Calldata = abi.encodeCall(
                IProofRewardPool.onCommitBlock,
                (_newBlocksData[i].blockNumber, _newBlocksData[i].newStateRoot, msg.sender)
            );

            if (i == _newBlocksData.length - 1) {
                fee = lastFee;
            }
            IMicro(microContract).onBlockEvent{value: fee}(msg.sender, l2Calldata, new bytes[](0));
        }
        _propagateToMicro();
    }

    /// @dev Make a call to the micro contract with the same calldata.
    /// Note: If the block is reverted, it needs to be committed first before the execution.
    /// So it's safe to not override the committed blocks.
    function revertBlocks(uint256) external onlyValidator {
        _propagateToMicro();
    }

    /// @dev Make a call to the micro contract with the same calldata.
    /// Note: We don't track the time when blocks are proven, since all information about
    /// the block is known on the commit stage and the proved is not finalized (may be reverted).
    function proveBlocks(
        StoredBlockInfo calldata,
        StoredBlockInfo[] calldata _newBlocksData,
        ProofInput calldata
    ) external payable onlyValidator {
        require(msg.value > 0, "proveBlocks msg.value is zero!");
        (uint256 fee, uint256 lastFee) = _splitFee(msg.value, _newBlocksData.length);

        for (uint256 i = 0; i < _newBlocksData.length; ++i) {
            bytes memory l2Calldata = abi.encodeCall(
                IProofRewardPool.onProveBlock,
                (_newBlocksData[i].blockNumber, _newBlocksData[i].blockHash, msg.sender)
            );
            if (i == _newBlocksData.length - 1) {
                fee = lastFee;
            }
            IMicro(microContract).onBlockEvent{value: fee}(msg.sender, l2Calldata, new bytes[](0));
        }
        _propagateToMicro();
    }

    /// @dev Check that blocks were committed at least X time ago and
    /// make a call to the micro contract with the same calldata.
    function executeBlocks(StoredBlockInfo[] calldata _newBlocksData) external payable onlyValidator {
        require(msg.value > 0, "executeBlocks msg.value is zero!");
        (uint256 fee, uint256 lastFee) = _splitFee(msg.value, _newBlocksData.length);

        for (uint256 i = 0; i < _newBlocksData.length; ++i) {
            uint256 commitBlockTimestamp = committedBlockTimestamp[_newBlocksData[i].blockNumber];

            bytes memory l2Calldata = abi.encodeCall(
                IProofRewardPool.onExecuteBlock,
                (_newBlocksData[i].blockNumber, _newBlocksData[i].blockHash, msg.sender)
            );
            if (i == _newBlocksData.length - 1) {
                fee = lastFee;
            }
            IMicro(microContract).onBlockEvent{value: fee}(msg.sender, l2Calldata, new bytes[](0));

            // Note: if the `commitBlockTimestamp` is zero, that means either:
            // * The block was committed, but not though this contract.
            // * The block wasn't committed at all, so execution will fail in the micro contract.
            // We allow executing such blocks.

            require(block.timestamp > commitBlockTimestamp + executionDelay, "5c"); // The delay is not passed
        }

        _propagateToMicro();
    }

    /// @dev Call the micro contract with the same calldata as this contract was called.
    /// Note: it is called the micro contract, not delegatecalled!
    function _propagateToMicro() internal {
        address contractAddress = microContract;
        assembly {
            // Copy function signature and arguments from calldata at zero position into memory at pointer position
            calldatacopy(0, 0, calldatasize())
            // Call method of the micro contract returns 0 on error
            let result := call(gas(), contractAddress, 0, 0, calldatasize(), 0, 0)
            // Get the size of the last return data
            let size := returndatasize()
            // Copy the size length of bytes from return data at zero position to pointer position
            returndatacopy(0, 0, size)
            // Depending on the result value
            switch result
            case 0 {
                // End execution and revert state changes
                revert(0, size)
            }
            default {
                // Return data with length of size at pointers position
                return(0, size)
            }
        }
    }

    function _splitFee(uint256 _totalFee, uint256 _length) internal pure returns (uint256, uint256) {
        uint256 fee = _totalFee / _length;
        uint256 lastFee = fee + (_totalFee - fee * _length);
        return (fee, lastFee);
    }
}
