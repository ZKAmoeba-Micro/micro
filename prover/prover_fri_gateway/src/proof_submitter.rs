use async_trait::async_trait;
use micro_dal::fri_proof_compressor_dal::ProofCompressionJobStatus;
use micro_types::{
    aggregated_operations::L1BatchProofForL1,
    prover_server_api::{SubmitProofRequest, SubmitProofResponse},
    L1BatchNumber, PackedEthSignature,
};

use crate::api_data_fetcher::{PeriodicApi, PeriodicApiStruct};

impl PeriodicApiStruct {
    async fn next_submit_proof_request(&self) -> Option<(L1BatchNumber, SubmitProofRequest)> {
        let (l1_batch_number, status) = self
            .pool
            .access_storage()
            .await
            .unwrap()
            .fri_proof_compressor_dal()
            .get_least_proven_block_number_not_sent_to_server()
            .await?;

        let request = match status {
            ProofCompressionJobStatus::Successful => {
                let mut l1_batch_proof: L1BatchProofForL1 = self
                    .blob_store
                    .get(l1_batch_number)
                    .await
                    .expect("Failed to get compressed snark proof from blob store");

                l1_batch_proof.sign(l1_batch_number, &self.config.prover_private_key().unwrap());
                SubmitProofRequest::Proof(Box::new(l1_batch_proof))
            }
            ProofCompressionJobStatus::Skipped => {
                let sign = PackedEthSignature::sign(
                    &self.config.prover_private_key().unwrap(),
                    &l1_batch_number.0.to_be_bytes(),
                )
                .unwrap();
                SubmitProofRequest::SkippedProofGeneration(sign)
            }
            _ => panic!(
                "Trying to send proof that are not successful status: {:?}",
                status
            ),
        };

        Some((l1_batch_number, request))
    }

    async fn save_successful_sent_proof(&self, l1_batch_number: L1BatchNumber) {
        self.pool
            .access_storage()
            .await
            .unwrap()
            .fri_proof_compressor_dal()
            .mark_proof_sent_to_server(l1_batch_number)
            .await;
    }
}

#[async_trait]
impl PeriodicApi<SubmitProofRequest> for PeriodicApiStruct {
    type JobId = L1BatchNumber;
    type Response = SubmitProofResponse;
    const SERVICE_NAME: &'static str = "ProofSubmitter";

    async fn get_next_request(&mut self) -> Option<(Self::JobId, SubmitProofRequest)> {
        let (l1_batch_number, request) = self.next_submit_proof_request().await?;
        Some((l1_batch_number, request))
    }

    async fn send_request(
        &self,
        job_id: Self::JobId,
        request: SubmitProofRequest,
    ) -> reqwest::Result<Self::Response> {
        let endpoint = format!("{}/{job_id}", self.api_url);
        self.send_http_request(request, &endpoint).await
    }

    async fn handle_response(&self, job_id: L1BatchNumber, response: Self::Response) {
        tracing::info!("Received response: {:?}", response);
        self.save_successful_sent_proof(job_id).await;
    }
}
