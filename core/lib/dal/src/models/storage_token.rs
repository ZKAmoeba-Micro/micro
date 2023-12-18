use micro_types::tokens::TokenPrice;
use micro_utils::big_decimal_to_ratio;
use sqlx::types::{
    chrono::{DateTime, NaiveDateTime, Utc},
    BigDecimal,
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StorageTokenPrice {
    pub usd_price: Option<BigDecimal>,
    pub usd_price_updated_at: Option<NaiveDateTime>,
}

impl From<StorageTokenPrice> for Option<TokenPrice> {
    fn from(price: StorageTokenPrice) -> Option<TokenPrice> {
        match (&price.usd_price, price.usd_price_updated_at) {
            (Some(usd_price), Some(updated_at)) => Some(TokenPrice {
                usd_price: big_decimal_to_ratio(usd_price).unwrap(),
                last_updated: DateTime::<Utc>::from_naive_utc_and_offset(updated_at, Utc),
            }),
            (None, None) => None,
            _ => {
                tracing::warn!(
                    "Found storage token with {:?} `usd_price` and {:?} `usd_price_updated_at`",
                    price.usd_price,
                    price.usd_price_updated_at
                );
                None
            }
        }
    }
}
