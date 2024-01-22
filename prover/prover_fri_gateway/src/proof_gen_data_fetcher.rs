use async_trait::async_trait;
use chrono::Utc;
use micro_prover_fri_utils::sync_status::get_sync_status;
use micro_types::{
    prover_server_api::{
        ProofGenerationData, ProofGenerationDataRequest, ProofGenerationDataResponse,
    },
    PackedEthSignature,
};

use crate::api_data_fetcher::{PeriodicApi, PeriodicApiStruct};

impl PeriodicApiStruct {
    async fn save_proof_gen_data(&self, data: ProofGenerationData) {
        let store = &*self.blob_store;
        let blob_url = store
            .put(data.l1_batch_number, &data.data)
            .await
            .expect("Failed to save proof generation data to GCS");
        let mut connection = self.pool.access_storage().await.unwrap();
        connection
            .fri_protocol_versions_dal()
            .save_prover_protocol_version(data.fri_protocol_version_id, data.l1_verifier_config)
            .await;
        connection
            .fri_witness_generator_dal()
            .save_witness_inputs(
                data.l1_batch_number,
                &blob_url,
                data.fri_protocol_version_id,
            )
            .await;
    }
}

#[async_trait]
impl PeriodicApi<ProofGenerationDataRequest> for PeriodicApiStruct {
    type JobId = ();
    type Response = ProofGenerationDataResponse;

    const SERVICE_NAME: &'static str = "ProofGenDataFetcher";

    async fn get_next_request(&mut self) -> Option<(Self::JobId, ProofGenerationDataRequest)> {
        if !self.check_sync_status {
            let sync_status = get_sync_status(self.pool.clone(), &self.rpc_url)
                .await
                .unwrap();
            if !sync_status {
                return None;
            }
            tracing::info!("Syncing is finished");
            self.check_sync_status = sync_status;
        }

        let timestamp = Utc::now().timestamp();
        let signature = PackedEthSignature::sign(
            &self.config.prover_private_key().unwrap(),
            &timestamp.to_be_bytes(),
        )
        .unwrap();
        Some((
            (),
            ProofGenerationDataRequest {
                timestamp,
                signature,
            },
        ))
    }

    async fn send_request(
        &self,
        _: (),
        request: ProofGenerationDataRequest,
    ) -> reqwest::Result<Self::Response> {
        self.send_http_request(request, &self.api_url).await
    }

    async fn handle_response(&self, _: (), response: Self::Response) {
        match response {
            ProofGenerationDataResponse::Success(None) => {
                tracing::info!("There are currently no pending batches to be proven");
            }
            ProofGenerationDataResponse::Success(Some(data)) => {
                tracing::info!("Received proof gen data for: {:?}", data.l1_batch_number);
                self.save_proof_gen_data(data).await;
            }
            ProofGenerationDataResponse::Error(err) => {
                tracing::error!("Failed to get proof gen data: {:?}", err);
            }
        }
    }
}
