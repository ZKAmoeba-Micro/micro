use anyhow::Context as _;

use tokio::{sync::oneshot, sync::watch};

use micro_config::configs::FriProverTaskApplyConfig;

use micro_utils::wait_for_tasks::wait_for_tasks;

use crate::client::MicroHttpQueryClient;
use crate::micro_watch::EthWatch;

use micro_eth_client::clients::http::QueryClient;

mod client;
mod micro_watch;

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

    let mut eth_watch = EthWatch::new(client, config).await;

    let tasks = vec![tokio::spawn(
        async move { eth_watch.run(stop_receiver).await },
    )];

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
