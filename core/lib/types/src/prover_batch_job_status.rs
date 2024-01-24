use micro_basic_types::L1BatchNumber;

pub struct ProverBatchJobStatus {
    pub l1_batch_number: L1BatchNumber,
    pub prover_status_successful_count: Option<u64>,
    pub prover_status_all_count: Option<u64>,
    pub compression_status: Option<String>,
}
