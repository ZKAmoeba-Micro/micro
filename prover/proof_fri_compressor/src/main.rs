use anyhow::Context as _;
use std::env;
use structopt::StructOpt;
use tokio::{sync::oneshot, sync::watch};

use std::time::Duration;

use micro_config::configs::FriProofCompressorConfig;
use micro_dal::connection::DbVariant;
use micro_dal::ConnectionPool;
use micro_object_store::ObjectStoreFactory;
use micro_queued_job_processor::JobProcessor;
use micro_utils::wait_for_tasks::wait_for_tasks;
use prometheus_exporter::PrometheusExporterConfig;

use crate::compressor::ProofCompressor;

mod compressor;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "micro_proof_fri_compressor",
    about = "Tool for compressing FRI proofs to old bellman proof"
)]
struct Opt {
    /// Number of times proof fri compressor should be run.
    #[structopt(short = "n", long = "n_iterations")]
    number_of_iterations: Option<usize>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let log_format = vlog::log_format_from_env();
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let sentry_url = vlog::sentry_url_from_env();
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let environment = vlog::environment_from_env();

    let mut builder = vlog::ObservabilityBuilder::new().with_log_format(log_format);
    if let Some(sentry_url) = sentry_url {
        builder = builder
            .with_sentry_url(&sentry_url)
            .context("Invalid Sentry URL")?
            .with_sentry_environment(environment);
    }
    let _guard = builder.build();

    let opt = Opt::from_args();
    let config = FriProofCompressorConfig::from_env().context("FriProofCompressorConfig")?;
    let pool = ConnectionPool::builder(DbVariant::Prover)
        .build()
        .await
        .context("failed to build a connection pool")?;
    let blob_store = ObjectStoreFactory::prover_from_env()
        .context("ObjectSToreFactor::prover_from_env()")?
        .create_store()
        .await;
    let proof_compressor = ProofCompressor::new(
        blob_store,
        pool,
        config.compression_mode,
        config.verify_wrapper_proof,
    );

    let (stop_sender, stop_receiver) = watch::channel(false);

    let (stop_signal_sender, stop_signal_receiver) = oneshot::channel();
    let mut stop_signal_sender = Some(stop_signal_sender);
    ctrlc::set_handler(move || {
        if let Some(stop_signal_sender) = stop_signal_sender.take() {
            stop_signal_sender.send(()).ok();
        }
    })
    .expect("Error setting Ctrl+C handler"); // Setting handler should always succeed.

    micro_prover_utils::ensure_initial_setup_keys_present(
        &config.universal_setup_path,
        &config.universal_setup_download_url,
    );
    env::set_var("CRS_FILE", config.universal_setup_path.clone());

    tracing::info!("Starting proof compressor");

    let mut tasks = vec![tokio::spawn(
        proof_compressor.run(stop_receiver.clone(), opt.number_of_iterations),
    )];

    if config.prometheus_listener_port != 0 {
        let prometheus_config = PrometheusExporterConfig::push(
            config.prometheus_pushgateway_url,
            Duration::from_millis(config.prometheus_push_interval_ms.unwrap_or(100)),
        );
        tasks.push(tokio::spawn(prometheus_config.run(stop_receiver)))
    }

    let graceful_shutdown = None::<futures::future::Ready<()>>;
    let tasks_allowed_to_finish = false;
    tokio::select! {
        _ = wait_for_tasks(tasks, None, graceful_shutdown, tasks_allowed_to_finish) => {},
        _ = stop_signal_receiver => {
            tracing::info!("Stop signal received, shutting down");
        }
    };
    stop_sender.send(true).ok();
    Ok(())
}
