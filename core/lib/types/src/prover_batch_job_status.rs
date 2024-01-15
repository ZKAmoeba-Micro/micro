use micro_basic_types::L1BatchNumber;

pub struct ProverBatchJobStatus {
    pub l1_batch_number: L1BatchNumber,
    pub prover_status_successful_count: u64,
    pub prover_status_all_count: u64,
    pub compression_status: String,
}
