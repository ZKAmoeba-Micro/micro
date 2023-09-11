use std::net::SocketAddr;
use std::time::Duration;

use micro_config::configs::api::Explorer as ExplorerApiConfig;
use micro_dal::connection::ConnectionPool;
use micro_types::Address;
use micro_utils::panic_notify::{spawn_panic_handler, ThreadPanicNotify};

use actix_cors::Cors;
use actix_web::dev::Server;
use actix_web::{web, App, HttpResponse, HttpServer};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use api_decl::RestApi;

pub mod api_decl;
pub mod api_impl;
pub mod network_stats;

fn start_server(api: RestApi, bind_to: SocketAddr, threads: usize) -> Server {
    HttpServer::new(move || {
        let api = api.clone();
        App::new()
            .wrap(
                Cors::default()
                    .send_wildcard()
                    .max_age(3600)
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method(),
            )
            .service(api.into_scope())
            // Endpoint needed for js isReachable
            .route(
                "/favicon.ico",
                web::get().to(|| async { HttpResponse::Ok().finish() }),
            )
    })
    .workers(threads)
    .bind(bind_to)
    .unwrap()
    .shutdown_timeout(60)
    .keep_alive(Duration::from_secs(10))
    .client_request_timeout(Duration::from_secs(60))
    .run()
}

/// Start HTTP REST API
pub fn start_server_thread_detached(
    api_config: ExplorerApiConfig,
    l2_erc20_bridge_addr: Address,
    fee_account_addr: Address,
    master_connection_pool: ConnectionPool,
    replica_connection_pool: ConnectionPool,
    mut stop_receiver: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let (handler, panic_sender) = spawn_panic_handler();

    std::thread::Builder::new()
        .name("explorer-api".to_string())
        .spawn(move || {
            let _panic_sentinel = ThreadPanicNotify(panic_sender.clone());

            actix_rt::System::new().block_on(async move {
                let bind_address = api_config.bind_addr();
                let threads = api_config.threads_per_server as usize;
                let api = RestApi::new(
                    master_connection_pool,
                    replica_connection_pool,
                    api_config,
                    l2_erc20_bridge_addr,
                    fee_account_addr,
                );
                api.spawn_network_stats_updater(panic_sender, stop_receiver.clone());

                let server = start_server(api, bind_address, threads);
                let close_handle = server.handle();
                actix_rt::spawn(async move {
                    if stop_receiver.changed().await.is_ok() {
                        close_handle.stop(true).await;
                        vlog::info!("Stop signal received, explorer API is shutting down");
                    }
                });
                server.await.expect("Explorer API crashed");
            });
        })
        .expect("Failed to spawn thread for REST API");

    handler
}
