use chrono::{DateTime, Utc};
use micro_basic_types::Address;
pub use micro_system_constants::ETHEREUM_ADDRESS;
use micro_utils::UnsignedRatioSerializeAsDecimal;
use num::{rational::Ratio, BigUint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TokenInfo {
    pub l1_address: Address,
    pub l2_address: Address,
    pub metadata: TokenMetadata,
}

/// Relevant information about tokens supported by micro protocol.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TokenMetadata {
    /// Token name (e.g. "Ethereum" or "USD Coin")
    pub name: String,
    /// Token symbol (e.g. "ETH" or "USDC")
    pub symbol: String,
    /// Token precision (e.g. 18 for "ETH" so "1.0" ETH = 10e18 as U256 number)
    pub decimals: u8,
}

impl TokenMetadata {
    /// Creates a default representation of token data, which will be used for tokens that have
    /// not known metadata.
    pub fn default(address: Address) -> Self {
        let default_name = format!("ERC20-{:x}", address);
        Self {
            name: default_name.clone(),
            symbol: default_name,
            decimals: 18,
        }
    }
}

/// Token price known to the micro network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    #[serde(with = "UnsignedRatioSerializeAsDecimal")]
    pub usd_price: Ratio<BigUint>,
    pub last_updated: DateTime<Utc>,
}
