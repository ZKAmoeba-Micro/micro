// Currently, every AGGR_* cost is overestimated,
// so there are safety margins around 100_000 -- 200_000

pub(super) const AGGR_L1_BATCH_COMMIT_BASE_COST: u64 = 60_000_000;
pub(super) const AGGR_L1_BATCH_PROVE_BASE_COST: u64 = 120_000_000;
pub(super) const AGGR_L1_BATCH_EXECUTE_BASE_COST: u64 = 160_000_000;

pub(super) const L1_BATCH_COMMIT_BASE_COST: u64 = 60_000_000;
pub(super) const L1_BATCH_PROVE_BASE_COST: u64 = 120_000_000;
pub(super) const L1_BATCH_EXECUTE_BASE_COST: u64 = 160_000_000;

pub(super) const EXECUTE_COMMIT_COST: u64 = 0;
pub(super) const EXECUTE_EXECUTE_COST: u64 = 0;

pub(super) const L1_OPERATION_EXECUTE_COST: u64 = 12_500;

pub(super) const GAS_PER_BYTE: u64 = 18;
