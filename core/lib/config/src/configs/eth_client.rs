// External uses
use serde::Deserialize;
// Local uses
use crate::envy_load;

/// Configuration for the Ethereum gateways.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ETHClientConfig {
    /// Numeric identifier of the L1 network (e.g. `9` for localhost).
    pub chain_id: u64,
    /// Address of the Ethereum node API.
    pub web3_url: String,
}

impl ETHClientConfig {
    pub fn from_env() -> Self {
        let config: Self = envy_load!("eth_client", "ETH_CLIENT_");
        if config.web3_url.find(',').is_some() {
            panic!(
                "Multiple web3 URLs aren't supported anymore. Provided invalid value: {}",
                config.web3_url
            );
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::test_utils::set_env;

    fn expected_config() -> ETHClientConfig {
        ETHClientConfig {
            chain_id: 9,
            web3_url: "http://127.0.0.1:1234/rpc/v1".into(),
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
ETH_CLIENT_CHAIN_ID="9"
ETH_CLIENT_WEB3_URL="http://127.0.0.1:1234/rpc/v1"
        "#;
        set_env(config);

        let actual = ETHClientConfig::from_env();
        assert_eq!(actual, expected_config());
    }
}
