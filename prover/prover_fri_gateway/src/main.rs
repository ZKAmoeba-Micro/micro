use anyhow::Context as _;
use micro_config::configs::{FriProverGatewayConfig, FriProverTaskApplyConfig, PostgresConfig};
use micro_dal::ConnectionPool;
use micro_env_config::{object_store::ProverObjectStoreConfig, FromEnv};
use micro_object_store::ObjectStoreFactory;
use micro_prover_fri_utils::app_monitor::{AppMonitor, AppMonitorJob};
use micro_types::prover_server_api::{ProofGenerationDataRequest, SubmitProofRequest};
use micro_utils::wait_for_tasks::wait_for_tasks;
use reqwest::Client;
use tokio::sync::{oneshot, watch};

use crate::api_data_fetcher::{PeriodicApiStruct, PROOF_GENERATION_DATA_PATH, SUBMIT_PROOF_PATH};

mod api_data_fetcher;
mod metrics;
mod proof_gen_data_fetcher;
mod proof_submitter;

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

    let config =
        FriProverGatewayConfig::from_env().context("FriProverGatewayConfig::from_env()")?;
    let task_apply_config =
        FriProverTaskApplyConfig::from_env().context("FriProverTaskApplyConfig::from_env()")?;
    let postgres_config = PostgresConfig::from_env().context("PostgresConfig::from_env()")?;
    let pool = ConnectionPool::builder(
        postgres_config.prover_url()?,
        postgres_config.max_connections()?,
    )
    .build()
    .await
    .context("failed to build a connection pool")?;
    let object_store_config =
        ProverObjectStoreConfig::from_env().context("ProverObjectStoreConfig::from_env()")?;
    let store_factory = ObjectStoreFactory::new(object_store_config.0);

    let mut tasks = vec![];
    let (stop_sender, stop_receiver) = watch::channel(false);

    let proof_submitter = PeriodicApiStruct {
        blob_store: store_factory.create_store().await,
        pool: pool.clone(),
        api_url: format!("{}{SUBMIT_PROOF_PATH}", config.api_url),
        rpc_url: task_apply_config.clone().rpc_url,
        poll_duration: config.api_poll_duration(),
        client: Client::new(),
        config: config.clone(),
        check_sync_status: false,
    };
    let proof_gen_data_fetcher = PeriodicApiStruct {
        blob_store: store_factory.create_store().await,
        pool,
        api_url: format!("{}{PROOF_GENERATION_DATA_PATH}", config.api_url),
        rpc_url: task_apply_config.rpc_url,
        poll_duration: config.api_poll_duration(),
        client: Client::new(),
        config: config.clone(),
        check_sync_status: false,
    };

    if let Some(url) = config.app_monitor_url {
        if let Some(interval) = config.retry_interval_ms {
            let app_monitor =
                AppMonitor::new("micro_prover_fri_gateway".to_string(), interval, url);
            tasks.push(tokio::spawn(app_monitor.run(stop_receiver.clone())));
        }
    }

    let (stop_signal_sender, stop_signal_receiver) = oneshot::channel();
    let mut stop_signal_sender = Some(stop_signal_sender);
    ctrlc::set_handler(move || {
        if let Some(stop_signal_sender) = stop_signal_sender.take() {
            stop_signal_sender.send(()).ok();
        }
    })
    .context("Error setting Ctrl+C handler")?;

    tracing::info!("Starting Fri Prover Gateway");

    tasks.push(tokio::spawn(
        proof_gen_data_fetcher.run::<ProofGenerationDataRequest>(stop_receiver.clone()),
    ));
    tasks.push(tokio::spawn(
        proof_submitter.run::<SubmitProofRequest>(stop_receiver.clone()),
    ));
    // tasks.push(tokio::spawn(
    //     PrometheusExporterConfig::pull(config.prometheus_listener_port).run(stop_receiver.clone()),
    // ));

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
