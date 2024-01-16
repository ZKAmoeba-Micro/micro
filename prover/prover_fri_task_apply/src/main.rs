use anyhow::Context as _;
use micro_config::configs::{FriProverTaskApplyConfig, PostgresConfig};
use micro_dal::ConnectionPool;
use micro_env_config::FromEnv;
use micro_eth_client::clients::http::QueryClient;
use micro_prover_fri_utils::app_monitor::{AppMonitor, AppMonitorJob};
use micro_utils::wait_for_tasks::wait_for_tasks;
use tokio::sync::{oneshot, watch};

use crate::{
    client::MicroHttpQueryClient, micro_watch::EthWatch, task_apply::TaskApply,
    wallet::TaskApplyWallet,
};

mod caller;
mod client;
mod micro_watch;
mod task_apply;
mod wallet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let log_format = vlog::log_format_from_env();

    let builder: vlog::ObservabilityBuilder =
        vlog::ObservabilityBuilder::new().with_log_format(log_format);

    let _guard = builder.build();

    std::env::vars().into_iter().for_each(|(k, v)| {
        println!("env {}={}", k, v);
    });

    let config =
        FriProverTaskApplyConfig::from_env().context("FriProverTaskApplyConfig::from_env()")?;
    let app_monitor_config = config.clone();

    let postgres_config = PostgresConfig::from_env().context("PostgresConfig::from_env()")?;
    let pool = ConnectionPool::builder(
        postgres_config.prover_url()?,
        postgres_config.max_connections()?,
    )
    .build()
    .await
    .context("failed to build a connection pool")?;

    let (stop_sender, stop_receiver) = watch::channel(false);

    let (stop_signal_sender, stop_signal_receiver) = oneshot::channel();
    let mut stop_signal_sender = Some(stop_signal_sender);
    ctrlc::set_handler(move || {
        if let Some(stop_signal_sender) = stop_signal_sender.take() {
            stop_signal_sender.send(()).ok();
        }
    })
    .context("Error setting Ctrl+C handler")?;

    tracing::info!("Starting Fri Prover TaskApply");

    let query_client = QueryClient::new(&config.rpc_url).unwrap();

    let client = MicroHttpQueryClient::new(query_client, Some(config.confirmations_for_eth_event));

    let wallet = TaskApplyWallet::new(config.clone()).await;
    let eth_watch_caller = wallet.get_caller();
    let task_apply_caller = wallet.get_caller();

    let mut eth_watch = EthWatch::new(client, config.clone(), eth_watch_caller).await;

    let mut task_apply = TaskApply::new(config, pool, task_apply_caller).await;

    let eth_watch_receiver = stop_receiver.clone();
    let task_apply_receiver = stop_receiver.clone();
    let app_monitor_receiver = stop_receiver.clone();

    let app_monitor = AppMonitor::new(
        "micro_prover_fri_gateway".to_string(),
        app_monitor_config.retry_interval_ms,
        app_monitor_config.app_monitor_url,
    );
    let tasks = vec![
        // tokio::spawn(async move { eth_watch.run(eth_watch_receiver).await }),
        tokio::spawn(async move { task_apply.run(task_apply_receiver).await }),
        tokio::spawn(async move { wallet.run(stop_receiver.clone()).await }),
        tokio::spawn(async move { app_monitor.run(app_monitor_receiver).await }),
    ];

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
