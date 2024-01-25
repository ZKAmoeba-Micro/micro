use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context as _;
use async_trait::async_trait;
use jsonrpc_core::types::response::Failure as RpcFailure;
use micro_types::app_monitor::Status;
use reqwest::Client;
use thiserror::Error;
use tokio::{
    sync::{watch, Mutex},
    time::sleep,
};

const ADD_URL: &str = "application/add";
const UPDATE_URL: &str = "application/update";
#[derive(Debug, Error, PartialEq)]
pub enum RpcError {
    #[error("Unable to decode server response")]
    MalformedResponse(String),
    #[error("RPC error: {0:?}")]
    RpcError(RpcFailure),
    #[error("Network error: {0}")]
    NetworkError(String),
}

#[derive(Debug, Clone)]
pub struct AppMonitor {
    app_name: String,
    retry_interval_ms: u64,
    client: Client,
    rpc_addr: String,
    start_time: i64,
    is_add: Option<bool>,
}

#[async_trait]
pub trait AppMonitorJob: Sync + Send {
    /// Runs the routine task periodically in [`Self::polling_interval_ms()`] frequency.
    async fn run_routine_task(&mut self) -> anyhow::Result<()>;
    async fn run(mut self, stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        loop {
            if *stop_receiver.borrow() {
                return Ok(());
            }
            self.run_routine_task()
                .await
                .context("AppMonitorJob run_routine_task fail")?;
            sleep(Duration::from_millis(self.polling_interval_ms())).await;
        }
    }
    fn polling_interval_ms(&self) -> u64;
}

fn timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("App_monitor Time went backwards");
    let ms = since_the_epoch.as_secs() as i64 * 1000i64
        + (since_the_epoch.subsec_nanos() as f64 / 1_000_000.0) as i64;
    ms
}

impl AppMonitor {
    pub fn new(app_name: String, retry_interval_ms: u64, rpc_addr: String) -> Self {
        let ts1 = timestamp();
        Self {
            app_name,
            retry_interval_ms,
            client: Client::new(),
            rpc_addr: rpc_addr,
            start_time: ts1,
            is_add: Some(false),
        }
    }
    async fn execute(&self, method: String) -> Option<String> {
        let mut ts1 = self.start_time;
        if method.eq(UPDATE_URL) {
            ts1 = timestamp();
        }
        let message = Status {
            app_name: self.app_name.clone(),
            start_time: self.start_time,
            heartbeat_update_at: ts1,
            heartbeat_time: self.retry_interval_ms as u32,
        };

        let result = Self::post_raw(
            method.to_string(),
            message.clone(),
            self.client.clone(),
            self.rpc_addr.clone(),
        )
        .await;
        match result {
            Ok(res) => {
                // tracing::info!(
                //     "app_monitor success app_name:{},message:{:?},res:{:#?}",
                //     &self.app_name,
                //     &message,
                //     res
                // );
                Some(res)
            }
            Err(e) => {
                tracing::error!(
                    "app_monitor  erro app_name:{},message:{:?},e:{}",
                    &self.app_name,
                    &message,
                    e
                );
                None
            }
        }
    }
    async fn post_raw(
        method: String,
        message: impl serde::Serialize,
        client: Client,
        rpc_addr: String,
    ) -> Result<String, RpcError> {
        let url = format!("{}/{}", &rpc_addr, method);
        let res = client
            .post(url)
            .json(&message)
            .send()
            .await
            .map_err(|err| RpcError::NetworkError(err.to_string()))?;
        if res.status() != reqwest::StatusCode::OK {
            let error = format!(
                "app_monitor Post query responded with a non-OK response: {}",
                res.status()
            );
            return Err(RpcError::NetworkError(error));
        }
        // let json_content = res.json::<serde_json::Value>().await;
        //let text_content = res.text().await;
        //tracing::info!("===========text_content:{:?}======",text_content);
        let reply = res
            .text()
            .await
            .map_err(|err| RpcError::MalformedResponse(err.to_string()))?;
        Ok(reply)
    }
}

#[async_trait]
impl AppMonitorJob for AppMonitor {
    async fn run_routine_task(&mut self) -> anyhow::Result<()> {
        match self.is_add {
            Some(t) => {
                if t {
                    let _res = self.execute(UPDATE_URL.to_string()).await;
                } else {
                    let res = self.execute(ADD_URL.to_string()).await;
                    match res {
                        Some(_) => {
                            self.is_add = Some(true);
                        }
                        None => {}
                    }
                }
            }
            None => {}
        }

        Ok(())
    }

    fn polling_interval_ms(&self) -> u64 {
        self.retry_interval_ms
    }
}
