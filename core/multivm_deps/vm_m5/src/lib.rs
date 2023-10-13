#![allow(clippy::derive_partial_eq_without_eq)]

mod bootloader_state;
pub mod errors;
pub mod event_sink;
mod events;
pub(crate) mod glue;
mod history_recorder;
pub mod legacy_types;
pub mod memory;
mod oracle_tools;
pub mod oracles;
mod pubdata_utils;
mod refunds;
pub mod storage;
pub mod test_utils;
pub mod transaction_data;
pub mod utils;
pub mod vm;
pub mod vm_with_bootloader;

#[cfg(test)]
mod tests;

pub use crate::errors::TxRevertReason;
pub use crate::oracle_tools::OracleTools;
pub use crate::oracles::storage::StorageOracle;
pub use crate::vm::VmBlockResult;
pub use crate::vm::VmExecutionResult;
pub use crate::vm::VmInstance;
pub use zk_evm;
pub use micro_types::vm_trace::VmExecutionTrace;

pub type Word = micro_types::U256;

pub const MEMORY_SIZE: usize = 1 << 16;
pub const MAX_CALLS: usize = 65536;
pub const REGISTERS_COUNT: usize = 16;
pub const MAX_STACK_SIZE: usize = 256;
pub const MAX_CYCLES_FOR_TX: u32 = u32::MAX;
