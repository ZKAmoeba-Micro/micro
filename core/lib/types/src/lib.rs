//! micro types: essential type definitions for micro network.
//!
//! `micro_types` is a crate containing essential micro network types, such as transactions, operations and
//! blockchain primitives.

#![allow(clippy::upper_case_acronyms, clippy::derive_partial_eq_without_eq)]

use fee::encoding_len;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub use crate::{Nonce, H256, U256, U64};

pub type SerialId = u64;

use crate::l2::TransactionType;
pub use event::{VmEvent, VmEventGroupKey};
pub use l1::L1TxCommonData;
pub use l2::L2TxCommonData;
pub use micro_basic_types::*;
pub use storage::*;
pub use tx::primitives::*;
pub use tx::Execute;
pub use zk_evm;
pub use zkevm_test_harness;

pub mod aggregated_operations;
pub mod block;
pub mod circuit;
pub mod commitment;
pub mod event;
pub mod explorer_api;
pub mod fee;
pub mod l1;
pub mod l2;
pub mod l2_to_l1_log;
pub mod priority_op_onchain_data;
pub mod pubdata_packing;
pub mod statistics_info;
pub mod storage;
pub mod storage_writes_deduplicator;
pub mod system_contracts;
pub mod tokens;
pub mod tx;
pub mod vm_trace;

pub mod api;
pub mod eth_sender;
pub mod helpers;
pub mod proofs;
pub mod transaction_request;
pub mod utils;
/// Denotes the first byte of the special micro's EIP-712-signed transaction.
pub const EIP_712_TX_TYPE: u8 = 0x71;

/// Denotes the first byte of the `EIP-1559` transaction.
pub const EIP_1559_TX_TYPE: u8 = 0x02;

/// Denotes the first byte of the `EIP-2930` transaction.
pub const EIP_2930_TX_TYPE: u8 = 0x01;

/// Denotes the first byte of some legacy transaction, which type is unknown to the server.
pub const LEGACY_TX_TYPE: u8 = 0x0;

/// Denotes the first byte of some legacy transaction, which type is unknown to the server.
pub const PRIORITY_OPERATION_L2_TX_TYPE: u8 = 0xff;

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub common_data: ExecuteTransactionCommon,
    pub execute: Execute,
    pub received_timestamp_ms: u64,
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Transaction").field(&self.hash()).finish()
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Transaction) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for Transaction {}

impl Transaction {
    /// Returns recipient account of the transaction.
    pub fn recipient_account(&self) -> Address {
        self.execute.contract_address
    }

    pub fn nonce(&self) -> Option<Nonce> {
        match &self.common_data {
            ExecuteTransactionCommon::L1(_) => None,
            ExecuteTransactionCommon::L2(tx) => Some(tx.nonce),
        }
    }

    pub fn is_l1(&self) -> bool {
        matches!(self.common_data, ExecuteTransactionCommon::L1(_))
    }

    pub fn tx_format(&self) -> TransactionType {
        match &self.common_data {
            ExecuteTransactionCommon::L1(tx) => tx.tx_format(),
            ExecuteTransactionCommon::L2(tx) => tx.transaction_type,
        }
    }

    pub fn type_display(&self) -> &'static str {
        match &self.common_data {
            ExecuteTransactionCommon::L1(_) => "l1_transaction",
            ExecuteTransactionCommon::L2(_) => "l2_transaction",
        }
    }

    pub fn hash(&self) -> H256 {
        match &self.common_data {
            ExecuteTransactionCommon::L1(data) => data.hash(),
            ExecuteTransactionCommon::L2(data) => data.hash(),
        }
    }

    /// Returns the account that initiated this transaction.
    pub fn initiator_account(&self) -> Address {
        match &self.common_data {
            ExecuteTransactionCommon::L1(data) => data.sender,
            ExecuteTransactionCommon::L2(data) => data.initiator_address,
        }
    }

    /// Returns the payer for L2 transaction and 0 for L1 transactions
    pub fn payer(&self) -> Address {
        match &self.common_data {
            ExecuteTransactionCommon::L1(data) => data.sender,
            ExecuteTransactionCommon::L2(data) => {
                let paymaster = data.paymaster_params.paymaster;
                if paymaster == Address::default() {
                    data.initiator_address
                } else {
                    paymaster
                }
            }
        }
    }

    pub fn gas_limit(&self) -> U256 {
        match &self.common_data {
            ExecuteTransactionCommon::L1(data) => data.gas_limit,
            ExecuteTransactionCommon::L2(data) => data.fee.gas_limit,
        }
    }

    pub fn max_fee_per_gas(&self) -> U256 {
        match &self.common_data {
            ExecuteTransactionCommon::L1(data) => data.max_fee_per_gas,
            ExecuteTransactionCommon::L2(data) => data.fee.max_fee_per_gas,
        }
    }

    pub fn gas_per_pubdata_byte_limit(&self) -> U256 {
        match &self.common_data {
            ExecuteTransactionCommon::L1(data) => data.gas_per_pubdata_limit,
            ExecuteTransactionCommon::L2(data) => data.fee.gas_per_pubdata_limit,
        }
    }

    // Returns how many slots it takes to encode the transaction
    pub fn encoding_len(&self) -> usize {
        let data_len = self.execute.calldata.len();
        let factory_deps_len = self
            .execute
            .factory_deps
            .as_ref()
            .map(|deps| deps.len())
            .unwrap_or_default();
        let (signature_len, paymaster_input_len) = match &self.common_data {
            ExecuteTransactionCommon::L1(_) => (0, 0),
            ExecuteTransactionCommon::L2(l2_common_data) => (
                l2_common_data.signature.len(),
                l2_common_data.paymaster_params.paymaster_input.len(),
            ),
        };

        encoding_len(
            data_len as u64,
            signature_len as u64,
            factory_deps_len as u64,
            paymaster_input_len as u64,
            0,
        )
    }
}

/// Optional input `Ethereum`-like encoded transaction if submitted via Web3 API.
/// If exists, its hash will be used to identify transaction.
/// Note, that for EIP712-type transactions, `hash` is not equal to the hash
/// of the `data`, but rather calculated by special formula.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputData {
    pub hash: H256,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecuteTransactionCommon {
    L1(L1TxCommonData),
    L2(L2TxCommonData),
}
