use std::io::copy;
use std::io::ErrorKind;
use std::io::Read;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::option::Option;
use std::time::Duration;
use std::time::Instant;

use local_ip_address::local_ip;
use prover_service::prover::{Prover, ProvingAssembly};
use prover_service::remote_synth::serialize_job;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use zkevm_test_harness::abstract_micro_circuit::concrete_circuits::MicroCircuit;
use zkevm_test_harness::bellman::plonk::better_better_cs::cs::Circuit;
use zkevm_test_harness::pairing::bn256::Bn256;
use zkevm_test_harness::witness::oracle::VmWitnessOracle;

use micro_config::configs::prover_group::ProverGroupConfig;
use micro_config::configs::CircuitSynthesizerConfig;
use micro_config::ProverConfigs;
use micro_dal::gpu_prover_queue_dal::{GpuProverInstanceStatus, SocketAddress};
use micro_dal::ConnectionPool;
use micro_object_store::{CircuitKey, ObjectStore, ObjectStoreError, ObjectStoreFactory};
use micro_prover_utils::numeric_index_to_circuit_name;
use micro_prover_utils::region_fetcher::{get_region, get_zone};
use micro_queued_job_processor::{async_trait, JobProcessor};

#[derive(Debug)]
pub enum CircuitSynthesizerError {
    InvalidGroupCircuits(u8),
    InvalidCircuitId(u8),
    InputLoadFailed(ObjectStoreError),
}

pub struct CircuitSynthesizer {
    config: CircuitSynthesizerConfig,
    blob_store: Box<dyn ObjectStore>,
    allowed_circuit_types: Option<Vec<String>>,
    region: String,
    zone: String,
}

impl CircuitSynthesizer {
    pub async fn new(
        config: CircuitSynthesizerConfig,
        prover_groups: ProverGroupConfig,
        store_factory: &ObjectStoreFactory,
    ) -> Result<Self, CircuitSynthesizerError> {
        let is_specialized = prover_groups.is_specialized_group_id(config.prover_group_id);
        let allowed_circuit_types = if is_specialized {
            let types = prover_groups
                .get_circuit_ids_for_group_id(config.prover_group_id)
                .ok_or(CircuitSynthesizerError::InvalidGroupCircuits(
                    config.prover_group_id,
                ))?
                .into_iter()
                .map(|id| {
                    numeric_index_to_circuit_name(id)
                        .map(|x| (id, x.to_owned()))
                        .ok_or(CircuitSynthesizerError::InvalidCircuitId(id))
                })
                .collect::<Result<Vec<_>, CircuitSynthesizerError>>()?;
            Some(types)
        } else {
            None
        };

        vlog::info!(
            "Configured for group [{}], circuits: {allowed_circuit_types:?}",
            config.prover_group_id
        );

        Ok(Self {
            config,
            blob_store: store_factory.create_store(),
            allowed_circuit_types: allowed_circuit_types
                .map(|x| x.into_iter().map(|x| x.1).collect()),
            region: get_region().await,
            zone: get_zone().await,
        })
    }

    pub fn synthesize(
        circuit: MicroCircuit<Bn256, VmWitnessOracle<Bn256>>,
    ) -> (ProvingAssembly, u8) {
        let start_instant = Instant::now();

        let mut assembly = Prover::new_proving_assembly();
        circuit
            .synthesize(&mut assembly)
            .expect("circuit synthesize failed");

        let circuit_type = numeric_index_to_circuit_name(circuit.numeric_circuit_type()).unwrap();

        vlog::info!(
            "Finished circuit synthesis for circuit: {circuit_type} took {:?}",
            start_instant.elapsed()
        );
        metrics::histogram!(
            "server.circuit_synthesizer.synthesize",
            start_instant.elapsed(),
            "circuit_type" => circuit_type,
        );

        // we don't perform assembly finalization here since it increases the assembly size significantly due to padding.
        (assembly, circuit.numeric_circuit_type())
    }
}

#[async_trait]
impl JobProcessor for CircuitSynthesizer {
    type Job = MicroCircuit<Bn256, VmWitnessOracle<Bn256>>;
    type JobId = u32;
    type JobArtifacts = (ProvingAssembly, u8);
    const SERVICE_NAME: &'static str = "CircuitSynthesizer";

    async fn get_next_job(
        &self,
        connection_pool: ConnectionPool,
    ) -> Option<(Self::JobId, Self::Job)> {
        vlog::trace!(
            "Attempting to fetch job types: {:?}",
            self.allowed_circuit_types
        );

        let mut storage = connection_pool.access_storage_blocking();
        let prover_job = match &self.allowed_circuit_types {
            Some(types) => storage
                .prover_dal()
                .get_next_prover_job_by_circuit_types(types.clone()),
            None => storage.prover_dal().get_next_prover_job(),
        }?;

        let circuit_key = CircuitKey {
            block_number: prover_job.block_number,
            sequence_number: prover_job.sequence_number,
            circuit_type: &prover_job.circuit_type,
            aggregation_round: prover_job.aggregation_round,
        };
        let input = self
            .blob_store
            .get(circuit_key)
            .map_err(CircuitSynthesizerError::InputLoadFailed)
            .unwrap_or_else(|err| panic!("{err:?}"));

        Some((prover_job.id, input))
    }

    async fn save_failure(
        &self,
        pool: ConnectionPool,
        job_id: Self::JobId,
        _started_at: Instant,
        error: String,
    ) {
        pool.access_storage_blocking()
            .prover_dal()
            .save_proof_error(job_id, error, self.config.max_attempts);
    }

    async fn process_job(
        &self,
        _connection_pool: ConnectionPool,
        job: Self::Job,
        _started_at: Instant,
    ) -> JoinHandle<Self::JobArtifacts> {
        tokio::task::spawn_blocking(move || Self::synthesize(job))
    }

    async fn save_result(
        &self,
        pool: ConnectionPool,
        job_id: Self::JobId,
        _started_at: Instant,
        (assembly, circuit_id): Self::JobArtifacts,
    ) {
        vlog::trace!(
            "Finished circuit synthesis for job: {job_id} in region: {}",
            self.region
        );

        let now = Instant::now();
        let mut serialized: Vec<u8> = vec![];
        serialize_job(&assembly, job_id as usize, circuit_id, &mut serialized);

        vlog::trace!(
            "Serialized circuit assembly for job {job_id} in {:?}",
            now.elapsed()
        );

        let now = Instant::now();
        let mut attempts = 0;

        while now.elapsed() < self.config.prover_instance_wait_timeout() {
            let prover = pool
                .access_storage_blocking()
                .gpu_prover_queue_dal()
                .lock_available_prover(
                    self.config.gpu_prover_queue_timeout(),
                    self.config.prover_group_id,
                    self.region.clone(),
                    self.zone.clone(),
                );

            if let Some(address) = prover {
                let result = send_assembly(job_id, &mut serialized, &address);
                handle_send_result(
                    &result,
                    job_id,
                    &address,
                    &pool,
                    self.region.clone(),
                    self.zone.clone(),
                );

                if result.is_ok() {
                    return;
                }
                // We'll retry with another prover again, no point in dropping the results.

                vlog::warn!(
                    "Could not send assembly to {address:?}. Prover group {}, region {}, \
                         circuit id {circuit_id}, send attempt {attempts}.",
                    self.config.prover_group_id,
                    self.region
                );
                attempts += 1;
            } else {
                sleep(self.config.prover_instance_poll_time()).await;
            }
        }
        vlog::trace!(
            "Not able to get any free prover instance for sending assembly for job: {job_id}"
        );
    }
}

fn send_assembly(
    job_id: u32,
    serialized: &mut Vec<u8>,
    address: &SocketAddress,
) -> Result<(Duration, u64), String> {
    vlog::trace!(
        "Sending assembly to {}:{}, job id {{{job_id}}}",
        address.host,
        address.port
    );

    let socket_address = SocketAddr::new(address.host, address.port);
    let started_at = Instant::now();
    let mut error_messages = vec![];

    for _ in 0..10 {
        match TcpStream::connect(socket_address) {
            Ok(mut stream) => {
                return send(&mut serialized.as_slice(), &mut stream)
                    .map(|result| (started_at.elapsed(), result))
                    .map_err(|err| format!("Could not send assembly to prover: {err:?}"));
            }
            Err(err) => {
                error_messages.push(format!("{err:?}"));
            }
        }
    }

    Err(format!(
        "Could not establish connection with prover after several attempts: {error_messages:?}"
    ))
}

fn send(read: &mut impl Read, tcp: &mut TcpStream) -> std::io::Result<u64> {
    let mut attempts = 10;
    let mut last_result = Ok(0);

    while attempts > 0 {
        match copy(read, tcp) {
            Ok(copied) => return Ok(copied),
            Err(err) if can_be_retried(err.kind()) => {
                attempts -= 1;
                last_result = Err(err);
            }
            Err(err) => return Err(err),
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    last_result
}

fn can_be_retried(err: ErrorKind) -> bool {
    matches!(err, ErrorKind::TimedOut | ErrorKind::ConnectionRefused)
}

fn handle_send_result(
    result: &Result<(Duration, u64), String>,
    job_id: u32,
    address: &SocketAddress,
    pool: &ConnectionPool,
    region: String,
    zone: String,
) {
    match result {
        Ok((elapsed, len)) => {
            let local_ip = local_ip().expect("Failed obtaining local IP address");
            let blob_size_in_gb = len / (1024 * 1024 * 1024);

            // region: logs

            vlog::trace!(
                "Sent assembly of size: {blob_size_in_gb}GB successfully, took: {elapsed:?} \
                 for job: {job_id} by: {local_ip:?} to: {address:?}"
            );
            metrics::histogram!(
                "server.circuit_synthesizer.blob_sending_time",
                *elapsed,
                "blob_size_in_gb" => blob_size_in_gb.to_string(),
            );

            // endregion

            pool.access_storage_blocking()
                .prover_dal()
                .update_status(job_id, "in_gpu_proof");
        }

        Err(err) => {
            vlog::trace!(
                "Failed sending assembly to address: {address:?}, socket not reachable \
                 reason: {err}"
            );

            // mark prover instance in gpu_prover_queue dead
            pool.access_storage_blocking()
                .gpu_prover_queue_dal()
                .update_prover_instance_status(
                    address.clone(),
                    GpuProverInstanceStatus::Dead,
                    0,
                    region,
                    zone,
                );

            let prover_config = ProverConfigs::from_env().non_gpu;
            // mark the job as failed
            pool.access_storage_blocking()
                .prover_dal()
                .save_proof_error(
                    job_id,
                    "prover instance unreachable".to_string(),
                    prover_config.max_attempts,
                );
        }
    }
}
