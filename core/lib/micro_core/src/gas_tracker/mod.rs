//! This module predicts L1 gas cost for the Commit/PublishProof/Execute operations.

use std::collections::HashMap;

use micro_types::{
    aggregated_operations::AggregatedActionType,
    block::{BlockGasCount, L1BatchHeader},
    commitment::{L1BatchMetadata, L1BatchWithMetadata},
    tx::tx_execution_info::{DeduplicatedWritesMetrics, ExecutionMetrics},
    ExecuteTransactionCommon, ProtocolVersionId, Transaction, H256,
};

mod constants;

use self::constants::*;

pub fn agg_l1_batch_base_cost(op: AggregatedActionType) -> u64 {
    match op {
        AggregatedActionType::Commit => AGGR_L1_BATCH_COMMIT_BASE_COST,
        AggregatedActionType::PublishProofOnchain => AGGR_L1_BATCH_PROVE_BASE_COST,
        AggregatedActionType::Execute => AGGR_L1_BATCH_EXECUTE_BASE_COST,
    }
}

pub fn l1_batch_base_cost(op: AggregatedActionType) -> u64 {
    match op {
        AggregatedActionType::Commit => L1_BATCH_COMMIT_BASE_COST,
        AggregatedActionType::PublishProofOnchain => L1_BATCH_PROVE_BASE_COST,
        AggregatedActionType::Execute => L1_BATCH_EXECUTE_BASE_COST,
    }
}

fn base_tx_cost(tx: &Transaction, op: AggregatedActionType) -> u64 {
    match op {
        AggregatedActionType::Commit => EXECUTE_COMMIT_COST,
        AggregatedActionType::PublishProofOnchain => 0,
        AggregatedActionType::Execute => match tx.common_data {
            ExecuteTransactionCommon::L1(_) => L1_OPERATION_EXECUTE_COST,
            ExecuteTransactionCommon::L2(_) => EXECUTE_EXECUTE_COST,
            ExecuteTransactionCommon::ProtocolUpgrade(_) => EXECUTE_EXECUTE_COST,
        },
    }
}

fn additional_pubdata_commit_cost(execution_metrics: &ExecutionMetrics) -> u64 {
    (execution_metrics.size() as u64) * GAS_PER_BYTE
}

fn additional_writes_commit_cost(
    writes_metrics: &DeduplicatedWritesMetrics,
    protocol_version: ProtocolVersionId,
) -> u64 {
    (writes_metrics.size(protocol_version) as u64) * GAS_PER_BYTE
}

pub fn new_block_gas_count() -> BlockGasCount {
    BlockGasCount {
        commit: l1_batch_base_cost(AggregatedActionType::Commit),
        prove: l1_batch_base_cost(AggregatedActionType::PublishProofOnchain),
        execute: l1_batch_base_cost(AggregatedActionType::Execute),
    }
}

pub fn gas_count_from_tx_and_metrics(
    tx: &Transaction,
    execution_metrics: &ExecutionMetrics,
) -> BlockGasCount {
    let commit = base_tx_cost(tx, AggregatedActionType::Commit)
        + additional_pubdata_commit_cost(execution_metrics);
    BlockGasCount {
        commit,
        prove: base_tx_cost(tx, AggregatedActionType::PublishProofOnchain),
        execute: base_tx_cost(tx, AggregatedActionType::Execute),
    }
}

pub fn gas_count_from_metrics(execution_metrics: &ExecutionMetrics) -> BlockGasCount {
    BlockGasCount {
        commit: additional_pubdata_commit_cost(execution_metrics),
        prove: 0,
        execute: 0,
    }
}

pub fn gas_count_from_writes(
    writes_metrics: &DeduplicatedWritesMetrics,
    protocol_version: ProtocolVersionId,
) -> BlockGasCount {
    BlockGasCount {
        commit: additional_writes_commit_cost(writes_metrics, protocol_version),
        prove: 0,
        execute: 0,
    }
}

pub(crate) fn commit_gas_count_for_l1_batch(
    header: &L1BatchHeader,
    unsorted_factory_deps: &HashMap<H256, Vec<u8>>,
    metadata: &L1BatchMetadata,
) -> u64 {
    let base_cost = l1_batch_base_cost(AggregatedActionType::Commit);
    let total_messages_len: u64 = header
        .l2_to_l1_messages
        .iter()
        .map(|message| message.len() as u64)
        .sum();
    let sorted_factory_deps =
        L1BatchWithMetadata::factory_deps_in_appearance_order(header, unsorted_factory_deps);
    let total_factory_deps_len: u64 = sorted_factory_deps
        .map(|factory_dep| factory_dep.len() as u64)
        .sum();

    // Boojum upgrade changes how storage writes are communicated/compressed.
    let is_pre_boojum = header
        .protocol_version
        .map(|v| v.is_pre_boojum())
        .unwrap_or(true);
    let state_diff_size = if is_pre_boojum {
        metadata.initial_writes_compressed.len() as u64
            + metadata.repeated_writes_compressed.len() as u64
    } else {
        metadata.state_diffs_compressed.len() as u64
    };

    let additional_calldata_bytes = state_diff_size
        + metadata.repeated_writes_compressed.len() as u64
        + metadata.l2_l1_messages_compressed.len() as u64
        + total_messages_len
        + total_factory_deps_len;
    let additional_cost = additional_calldata_bytes * GAS_PER_BYTE;
    base_cost + additional_cost
}
