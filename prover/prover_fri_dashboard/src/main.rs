use std::{net::SocketAddr, sync::Arc};

use anyhow::Context as _;
use axum::{
    routing::{get, post},
    Router,
};
use dashboard::Dashboard;
use micro_config::{
    configs::{fri_prover_dashboard::FriProverDashboardConfig, FriProverTaskApplyConfig},
    PostgresConfig,
};
use micro_dal::ConnectionPool;
use micro_env_config::FromEnv;
use micro_web3_decl::jsonrpsee::http_client::HttpClientBuilder;
use tower_http::services::ServeDir;

mod application;
mod application_monitor;
mod dashboard;
mod deposit;
mod error;
mod node;
mod task;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[allow(deprecated)] // TODO (QIT-21): Use centralized configuration approach.
    let log_format = vlog::log_format_from_env();

    let builder: vlog::ObservabilityBuilder =
        vlog::ObservabilityBuilder::new().with_log_format(log_format);

    let _guard = builder.build();
    tracing::info!("start Dashboard");
    let config =
        FriProverDashboardConfig::from_env().context("FriProverDashboardConfig::from_env()")?;

    let postgres_config = PostgresConfig::from_env().context("PostgresConfig::from_env()")?;
    let pool = ConnectionPool::builder(
        postgres_config.prover_url()?,
        postgres_config.max_connections()?,
    )
    .build()
    .await
    .context("failed to build a connection pool")?;

    let task_apply_config =
        FriProverTaskApplyConfig::from_env().context("FriProverTaskApplyConfig::from_env()")?;

    let client = HttpClientBuilder::default()
        .build(task_apply_config.clone().rpc_url)
        .expect("faile to build rpc client");

    let app_state = Arc::new(Dashboard {
        pool,
        client,
        config: task_apply_config,
    });

    let app = Router::new()
        .route("/deposit", get(deposit::get))
        .route("/node", get(node::get))
        .route("/tasks", get(task::get))
        .route("/application", get(application::get))
        .route("/application/add", post(application::add))
        .route("/application/update", post(application::update))
        .nest_service("/", ServeDir::new("/ui"))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
