use std::{convert::TryFrom, sync::Arc};

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use micro_config::configs::{
    proof_data_handler::ProtocolVersionLoadingMode, ProofDataHandlerConfig,
};
use micro_dal::{assignments_dal::ProverResultStatus, ConnectionPool, SqlxError};
use micro_object_store::{ObjectStore, ObjectStoreError};
use micro_types::{
    commitment::serialize_commitments,
    protocol_version::{FriProtocolVersionId, L1VerifierConfig},
    prover_server_api::{
        ProofGenerationData, ProofGenerationDataRequest, ProofGenerationDataResponse,
        SubmitProofRequest, SubmitProofResponse,
    },
    web3::signing::keccak256,
    L1BatchNumber, PackedEthSignature, H256,
};
use micro_utils::u256_to_h256;

#[derive(Clone)]
pub(crate) struct RequestProcessor {
    blob_store: Arc<dyn ObjectStore>,
    pool: ConnectionPool,
    config: ProofDataHandlerConfig,
    l1_verifier_config: Option<L1VerifierConfig>,
}

pub(crate) enum RequestProcessorError {
    ObjectStore(ObjectStoreError),
    Sqlx(SqlxError),
    SignatureError,
    ProveTimeout,
    InvalidProof,
}

impl IntoResponse for RequestProcessorError {
    fn into_response(self) -> Response {
        let (status_code, message) = match self {
            RequestProcessorError::ObjectStore(err) => {
                tracing::error!("GCS error: {:?}", err);
                (
                    StatusCode::BAD_GATEWAY,
                    "Failed fetching/saving from GCS".to_owned(),
                )
            }
            RequestProcessorError::Sqlx(err) => {
                tracing::error!("Sqlx error: {:?}", err);
                match err {
                    SqlxError::RowNotFound => {
                        (StatusCode::NOT_FOUND, "Non existing L1 batch".to_owned())
                    }
                    _ => (
                        StatusCode::BAD_GATEWAY,
                        "Failed fetching/saving from db".to_owned(),
                    ),
                }
            }
            RequestProcessorError::SignatureError => {
                (StatusCode::BAD_REQUEST, "Invalid signature".to_owned())
            }
            RequestProcessorError::ProveTimeout => {
                (StatusCode::BAD_REQUEST, "Prove timeout".to_owned())
            }
            RequestProcessorError::InvalidProof => {
                (StatusCode::BAD_REQUEST, "Invalid proof".to_owned())
            }
        };
        tracing::info!("proof response {} {}", status_code, message);
        (status_code, message).into_response()
    }
}

impl RequestProcessor {
    pub(crate) fn new(
        blob_store: Box<dyn ObjectStore>,
        pool: ConnectionPool,
        config: ProofDataHandlerConfig,
        l1_verifier_config: Option<L1VerifierConfig>,
    ) -> Self {
        Self {
            blob_store: Arc::from(blob_store),
            pool,
            config,
            l1_verifier_config,
        }
    }

    pub(crate) async fn get_proof_generation_data(
        &self,
        request: Json<ProofGenerationDataRequest>,
    ) -> Result<Json<ProofGenerationDataResponse>, RequestProcessorError> {
        tracing::info!("Received request for proof generation data: {:?}", request);

        // recover prover address
        let prover_addr = request
            .signature
            .signature_recover_signer(&PackedEthSignature::message_to_signed_bytes(
                &request.timestamp.to_be_bytes(),
            ))
            .map_err(|_| RequestProcessorError::SignatureError)?;
        if prover_addr.is_zero() {
            return Err(RequestProcessorError::SignatureError);
        }

        tracing::info!(
            "Received request for proof generation data prover: {:?}",
            prover_addr
        );

        // get proof job by prover address
        let l1_batch_number_result = self
            .pool
            .access_storage()
            .await
            .unwrap()
            .assignments_dal()
            .get_next_block_to_be_proven(prover_addr)
            .await;
        let l1_batch_number = match l1_batch_number_result {
            Some(number) => number,
            None => return Ok(Json(ProofGenerationDataResponse::Success(None))), // no batches pending to be proven
        };

        let blob = self
            .blob_store
            .get(l1_batch_number)
            .await
            .map_err(RequestProcessorError::ObjectStore)?;

        let fri_protocol_version_id =
            FriProtocolVersionId::try_from(self.config.fri_protocol_version_id)
                .expect("Invalid FRI protocol version id");

        let l1_verifier_config= match self.config.protocol_version_loading_mode {
            ProtocolVersionLoadingMode::FromDb => {
                panic!("Loading protocol version from db is not implemented yet")
            }
            ProtocolVersionLoadingMode::FromEnvVar => {
                self.l1_verifier_config
                    .expect("l1_verifier_config must be set while running ProtocolVersionLoadingMode::FromEnvVar mode")
            }
        };

        let proof_gen_data = ProofGenerationData {
            l1_batch_number,
            data: blob,
            fri_protocol_version_id,
            l1_verifier_config,
        };

        Ok(Json(ProofGenerationDataResponse::Success(Some(
            proof_gen_data,
        ))))
    }

    pub(crate) async fn submit_proof(
        &self,
        Path(l1_batch_number): Path<u32>,
        Json(payload): Json<SubmitProofRequest>,
    ) -> Result<Json<SubmitProofResponse>, RequestProcessorError> {
        tracing::info!("Received proof for block number: {:?}", l1_batch_number);
        let l1_batch_number = L1BatchNumber(l1_batch_number);
        match payload {
            SubmitProofRequest::Proof(mut proof) => {
                // recover prover address
                let prover_addr = proof
                    .signature_recover_signer(l1_batch_number)
                    .map_err(|_| RequestProcessorError::SignatureError)?;
                if prover_addr.is_zero() {
                    return Err(RequestProcessorError::SignatureError);
                }

                let mut storage = self.pool.access_storage().await.unwrap();

                // check job status and timeout
                let job = storage
                    .assignments_dal()
                    .get_job_details(prover_addr, l1_batch_number)
                    .await
                    .map_err(RequestProcessorError::Sqlx)?;

                if let Some((status, created_at)) = job {
                    match status {
                        ProverResultStatus::PickedByProver => {
                            let now = Utc::now().timestamp();
                            let time_taken = now - created_at;
                            proof.time_taken = time_taken as u64;

                            if time_taken > self.config.proof_generation_timeout_in_secs as i64 {
                                return Err(RequestProcessorError::ProveTimeout);
                            }
                        }
                        ProverResultStatus::Rollbacked => {
                            return Ok(Json(SubmitProofResponse::Success));
                        }
                        _ => {
                            return Ok(Json(SubmitProofResponse::Success));
                        }
                    }
                } else {
                    return Ok(Json(SubmitProofResponse::Success));
                }

                let blob_url = self
                    .blob_store
                    .put(l1_batch_number, &*proof)
                    .await
                    .map_err(RequestProcessorError::ObjectStore)?;

                let system_logs_hash_from_prover =
                    H256::from_slice(&proof.aggregation_result_coords[0]);
                let state_diff_hash_from_prover =
                    H256::from_slice(&proof.aggregation_result_coords[1]);
                let bootloader_heap_initial_content_from_prover =
                    H256::from_slice(&proof.aggregation_result_coords[2]);
                let events_queue_state_from_prover =
                    H256::from_slice(&proof.aggregation_result_coords[3]);

                let l1_batch = storage
                    .blocks_dal()
                    .get_l1_batch_metadata(l1_batch_number)
                    .await
                    .unwrap()
                    .expect("Proved block without metadata");

                let is_pre_boojum = l1_batch
                    .header
                    .protocol_version
                    .map(|v| v.is_pre_boojum())
                    .unwrap_or(true);
                if !is_pre_boojum {
                    let events_queue_state = l1_batch
                        .metadata
                        .events_queue_commitment
                        .expect("No events_queue_commitment");
                    let bootloader_heap_initial_content = l1_batch
                        .metadata
                        .bootloader_initial_content_commitment
                        .expect("No bootloader_initial_content_commitment");

                    if events_queue_state != events_queue_state_from_prover
                        || bootloader_heap_initial_content
                            != bootloader_heap_initial_content_from_prover
                    {
                        let server_values = format!("events_queue_state = {events_queue_state}, bootloader_heap_initial_content = {bootloader_heap_initial_content}");
                        let prover_values = format!("events_queue_state = {events_queue_state_from_prover}, bootloader_heap_initial_content = {bootloader_heap_initial_content_from_prover}");
                        tracing::error!(
                            "Auxilary output doesn't match, server values: {} prover values: {}",
                            server_values,
                            prover_values
                        );
                        return Err(RequestProcessorError::InvalidProof);
                    }
                }

                let system_logs = serialize_commitments(&l1_batch.header.system_logs);
                let system_logs_hash = H256(keccak256(&system_logs));

                if !is_pre_boojum {
                    let state_diff_hash = l1_batch
                        .header
                        .system_logs
                        .into_iter()
                        .find(|elem| elem.0.key == u256_to_h256(2.into()))
                        .expect("No state diff hash key")
                        .0
                        .value;

                    if state_diff_hash != state_diff_hash_from_prover
                        || system_logs_hash != system_logs_hash_from_prover
                    {
                        let server_values = format!("system_logs_hash = {system_logs_hash}, state_diff_hash = {state_diff_hash}");
                        let prover_values = format!("system_logs_hash = {system_logs_hash_from_prover}, state_diff_hash = {state_diff_hash_from_prover}");
                        tracing::error!(
                            "Auxilary output doesn't match, server values: {} prover values: {}",
                            server_values,
                            prover_values
                        );
                        return Err(RequestProcessorError::InvalidProof);
                    }
                }
                storage
                    .assignments_dal()
                    .save_proof_artifacts_metadata(l1_batch_number, &blob_url)
                    .await
                    .map_err(RequestProcessorError::Sqlx)?;
            }
            SubmitProofRequest::SkippedProofGeneration(signature) => {
                let signed_bytes =
                    PackedEthSignature::message_to_signed_bytes(&l1_batch_number.0.to_be_bytes());
                let prover_addr = signature
                    .signature_recover_signer(&signed_bytes)
                    .map_err(|_| RequestProcessorError::SignatureError)?;
                if prover_addr.is_zero() {
                    return Err(RequestProcessorError::SignatureError);
                }

                // do nothing, wait for timeout

                // self.pool
                //     .access_storage()
                //     .await
                //     .unwrap()
                //     .proof_generation_dal()
                //     .mark_proof_generation_job_as_skipped(l1_batch_number)
                //     .await
                //     .map_err(RequestProcessorError::Sqlx)?;
            }
        }

        Ok(Json(SubmitProofResponse::Success))
    }
}
