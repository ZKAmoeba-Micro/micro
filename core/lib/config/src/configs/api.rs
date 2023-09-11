/// External uses
use serde::Deserialize;
/// Built-in uses
use std::net::SocketAddr;
use std::time::Duration;
// Local uses
pub use crate::configs::utils::Prometheus;
use crate::envy_load;
use micro_basic_types::H256;

/// API configuration.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ApiConfig {
    /// Configuration options for the Web3 JSON RPC servers.
    pub web3_json_rpc: Web3JsonRpc,
    /// Configuration options for the REST servers.
    pub explorer: Explorer,
    /// Configuration options for the Prometheus exporter.
    pub prometheus: Prometheus,
    /// Configuration options for the Health check.
    pub healthcheck: HealthCheck,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self {
            web3_json_rpc: envy_load!("web3_json_rpc", "API_WEB3_JSON_RPC_"),
            explorer: envy_load!("explorer", "API_EXPLORER_"),
            prometheus: envy_load!("prometheus", "API_PROMETHEUS_"),
            healthcheck: envy_load!("healthcheck", "API_HEALTHCHECK_"),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Web3JsonRpc {
    /// Port to which the HTTP RPC server is listening.
    pub http_port: u16,
    /// URL to access HTTP RPC server.
    pub http_url: String,
    /// Port to which the WebSocket RPC server is listening.
    pub ws_port: u16,
    /// URL to access WebSocket RPC server.
    pub ws_url: String,
    /// Max possible limit of entities to be requested once.
    pub req_entities_limit: Option<u32>,
    /// Max possible limit of filters to be in the state at once.
    pub filters_limit: Option<u32>,
    /// Max possible limit of subscriptions to be in the state at once.
    pub subscriptions_limit: Option<u32>,
    /// Interval between polling db for pubsub (in ms).
    pub pubsub_polling_interval: Option<u64>,
    /// number of threads per server
    pub threads_per_server: u32,
    /// Tx nonce: how far ahead from the committed nonce can it be.
    pub max_nonce_ahead: u32,
    /// The multiplier to use when suggesting gas price. Should be higher than one,
    /// otherwise if the L1 prices soar, the suggested gas price won't be sufficient to be included in block
    pub gas_price_scale_factor: f64,
    /// Inbound transaction limit used for throttling
    pub transactions_per_sec_limit: Option<u32>,
    /// Timeout for requests (in s)
    pub request_timeout: Option<u64>,
    /// Private keys for accounts managed by node
    pub account_pks: Option<Vec<H256>>,
    /// The factor by which to scale the gasLimit
    pub estimate_gas_scale_factor: f64,
    /// The max possible number of gas that `eth_estimateGas` is allowed to overestimate.
    pub estimate_gas_acceptable_overestimation: u32,
    ///  Max possible size of an ABI encoded tx (in bytes).
    pub max_tx_size: usize,
    /// Main node URL - used only by external node to proxy transactions to.
    pub main_node_url: Option<String>,
    /// Max number of cache misses during one VM execution. If the number of cache misses exceeds this value, the api server panics.
    /// This is a temporary solution to mitigate API request resulting in thousands of DB queries.
    pub vm_execution_cache_misses_limit: Option<usize>,
}

impl Web3JsonRpc {
    pub fn http_bind_addr(&self) -> SocketAddr {
        SocketAddr::new("0.0.0.0".parse().unwrap(), self.http_port)
    }

    pub fn ws_bind_addr(&self) -> SocketAddr {
        SocketAddr::new("0.0.0.0".parse().unwrap(), self.ws_port)
    }

    pub fn req_entities_limit(&self) -> usize {
        self.req_entities_limit.unwrap_or_else(|| 2u32.pow(10)) as usize
    }

    pub fn filters_limit(&self) -> usize {
        self.filters_limit.unwrap_or(10000) as usize
    }

    pub fn subscriptions_limit(&self) -> usize {
        self.subscriptions_limit.unwrap_or(10000) as usize
    }

    pub fn pubsub_interval(&self) -> Duration {
        Duration::from_millis(self.pubsub_polling_interval.unwrap_or(200))
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout.unwrap_or(10))
    }

    pub fn account_pks(&self) -> Vec<H256> {
        self.account_pks.clone().unwrap_or_default()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct HealthCheck {
    /// Port to which the REST server is listening.
    pub port: u16,
}

impl HealthCheck {
    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new("0.0.0.0".parse().unwrap(), self.port)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Explorer {
    /// Port to which the REST server is listening.
    pub port: u16,
    /// URL to access REST server.
    pub url: String,
    /// Interval between polling db for network stats (in ms).
    pub network_stats_polling_interval: Option<u64>,
    /// Max possible limit of entities to be requested once.
    pub req_entities_limit: Option<u32>,
    /// Max possible value of (offset + limit) in pagination endpoints.
    pub offset_limit: Option<u32>,
    /// number of threads per server
    pub threads_per_server: u32,
}

impl Explorer {
    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new("0.0.0.0".parse().unwrap(), self.port)
    }

    pub fn network_stats_interval(&self) -> Duration {
        Duration::from_millis(self.network_stats_polling_interval.unwrap_or(1000))
    }

    pub fn req_entities_limit(&self) -> usize {
        self.req_entities_limit.unwrap_or(100) as usize
    }

    pub fn offset_limit(&self) -> usize {
        self.offset_limit.unwrap_or(10000) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::test_utils::set_env;
    use std::net::IpAddr;
    use std::str::FromStr;

    fn expected_config() -> ApiConfig {
        ApiConfig {
            web3_json_rpc: Web3JsonRpc {
                http_port: 3050,
                http_url: "http://127.0.0.1:3050".into(),
                ws_port: 3051,
                ws_url: "ws://127.0.0.1:3051".into(),
                req_entities_limit: Some(10000),
                filters_limit: Some(10000),
                subscriptions_limit: Some(10000),
                pubsub_polling_interval: Some(200),
                threads_per_server: 128,
                max_nonce_ahead: 5,
                transactions_per_sec_limit: Some(1000),
                request_timeout: Some(10),
                account_pks: Some(vec![
                    H256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000001",
                    )
                    .unwrap(),
                    H256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000002",
                    )
                    .unwrap(),
                ]),
                estimate_gas_scale_factor: 1.0f64,
                gas_price_scale_factor: 1.2,
                estimate_gas_acceptable_overestimation: 1000,
                max_tx_size: 1000000,
                main_node_url: None,
                vm_execution_cache_misses_limit: None,
            },
            explorer: Explorer {
                port: 3070,
                url: "http://127.0.0.1:3070".into(),
                network_stats_polling_interval: Some(1000),
                req_entities_limit: Some(100),
                offset_limit: Some(10000),
                threads_per_server: 128,
            },
            prometheus: Prometheus {
                listener_port: 3312,
                pushgateway_url: "http://127.0.0.1:9091".into(),
                push_interval_ms: Some(100),
            },
            healthcheck: HealthCheck { port: 8081 },
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
API_WEB3_JSON_RPC_HTTP_PORT="3050"
API_WEB3_JSON_RPC_HTTP_URL="http://127.0.0.1:3050"
API_WEB3_JSON_RPC_WS_PORT="3051"
API_WEB3_JSON_RPC_WS_URL="ws://127.0.0.1:3051"
API_WEB3_JSON_RPC_REQ_ENTITIES_LIMIT=10000
API_WEB3_JSON_RPC_FILTERS_LIMIT=10000
API_WEB3_JSON_RPC_SUBSCRIPTIONS_LIMIT=10000
API_WEB3_JSON_RPC_PUBSUB_POLLING_INTERVAL=200
API_WEB3_JSON_RPC_THREADS_PER_SERVER=128
API_WEB3_JSON_RPC_MAX_NONCE_AHEAD=5
API_WEB3_JSON_RPC_GAS_PRICE_SCALE_FACTOR=1.2
API_WEB3_JSON_RPC_TRANSACTIONS_PER_SEC_LIMIT=1000
API_WEB3_JSON_RPC_REQUEST_TIMEOUT=10
API_WEB3_JSON_RPC_ACCOUNT_PKS=0x0000000000000000000000000000000000000000000000000000000000000001,0x0000000000000000000000000000000000000000000000000000000000000002
API_WEB3_JSON_RPC_ESTIMATE_GAS_SCALE_FACTOR=1.0
API_WEB3_JSON_RPC_ESTIMATE_GAS_ACCEPTABLE_OVERESTIMATION=1000
API_WEB3_JSON_RPC_MAX_TX_SIZE=1000000
API_EXPLORER_PORT="3070"
API_EXPLORER_URL="http://127.0.0.1:3070"
API_EXPLORER_NETWORK_STATS_POLLING_INTERVAL="1000"
API_EXPLORER_REQ_ENTITIES_LIMIT=100
API_EXPLORER_OFFSET_LIMIT=10000
API_EXPLORER_THREADS_PER_SERVER=128
API_PROMETHEUS_LISTENER_PORT="3312"
API_PROMETHEUS_PUSHGATEWAY_URL="http://127.0.0.1:9091"
API_PROMETHEUS_PUSH_INTERVAL_MS=100
API_HEALTHCHECK_PORT=8081
        "#;
        set_env(config);

        let actual = ApiConfig::from_env();
        assert_eq!(actual, expected_config());
    }

    /// Checks the correctness of the config helper methods.
    #[test]
    fn methods() {
        let config = expected_config();
        let bind_broadcast_addr: IpAddr = "0.0.0.0".parse().unwrap();

        assert_eq!(
            config.web3_json_rpc.pubsub_interval(),
            Duration::from_millis(200)
        );
        assert_eq!(
            config.explorer.bind_addr(),
            SocketAddr::new(bind_broadcast_addr, config.explorer.port)
        );
    }
}
