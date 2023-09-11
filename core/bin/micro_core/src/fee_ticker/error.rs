use micro_types::Address;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TickerError {
    #[error("Token {0:x} is not being tracked for its price")]
    PriceNotTracked(Address),
    #[error("Third-party API data is temporarily unavailable")]
    ApiDataUnavailable,
    #[error("Fee ticker internal error")]
    InternalError,
}
