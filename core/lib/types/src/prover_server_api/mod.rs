use serde::{Deserialize, Serialize};

use micro_basic_types::L1BatchNumber;

use crate::aggregated_operations::L1BatchProofForL1;
use crate::proofs::PrepareBasicCircuitsJob;
use crate::protocol_version::{FriProtocolVersionId, L1VerifierConfig};
use crate::PackedEthSignature;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofGenerationData {
    pub l1_batch_number: L1BatchNumber,
    pub data: PrepareBasicCircuitsJob,
    pub fri_protocol_version_id: FriProtocolVersionId,
    pub l1_verifier_config: L1VerifierConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofGenerationDataRequest {
    pub timestamp: i64,
    pub signature: PackedEthSignature,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProofGenerationDataResponse {
    Success(ProofGenerationData),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitProofRequest {
    Proof(Box<L1BatchProofForL1>),
    // The proof generation was skipped due to sampling
    SkippedProofGeneration,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubmitProofResponse {
    Success,
    Error(String),
}
