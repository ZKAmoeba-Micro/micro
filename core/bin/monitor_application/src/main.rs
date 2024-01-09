use anyhow::Context as _;
use micro_config::{configs::ProofDataHandlerConfig, PostgresConfig};
use micro_core::monitor_application::monitor_transactions::MonitorTransactions;
use micro_dal::ConnectionPool;
use micro_env_config::FromEnv;
use micro_utils::wait_for_tasks::wait_for_tasks;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let log_format = vlog::log_format_from_env();
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let sentry_url = vlog::sentry_url_from_env();
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let environment = vlog::environment_from_env();

    let mut builder = vlog::ObservabilityBuilder::new().with_log_format(log_format);
    if let Some(sentry_url) = &sentry_url {
        builder = builder
            .with_sentry_url(sentry_url)
            .expect("Invalid Sentry URL")
            .with_sentry_environment(environment);
    }
    let _guard = builder.build();
    tracing::info!("Starting monitor creator");
    let config = PostgresConfig::from_env().unwrap();
    let db_url = config.replica_url()?;
    let pool = ConnectionPool::singleton(db_url)
        .build()
        .await
        .context("failed to build connection_pool")?;

    let proof_data_handler =
        ProofDataHandlerConfig::from_env().context("proof_data_handler_config")?;
    let (stop_sender, stop_receiver) = watch::channel(false);

    let monitor = MonitorTransactions::new(
        pool,
        proof_data_handler.proof_generation_timeout(),
        proof_data_handler.retry_interval_ms,
    );
    let tasks = vec![tokio::spawn(monitor.run(stop_receiver))];

    let particular_crypto_alerts = None;
    let graceful_shutdown = None::<futures::future::Ready<()>>;
    let tasks_allowed_to_finish = false;
    tokio::select! {
        _ = wait_for_tasks(tasks, particular_crypto_alerts, graceful_shutdown, tasks_allowed_to_finish) => {},
    };
    let _ = stop_sender.send(true);

    // Sleep for some time to let verifier gracefully stop.
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    tracing::info!("Finished running monitor creator!");
    Ok(())
}
