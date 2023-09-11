//! This module provides several third-party API data fetchers.
//! Examples of fetchers we use:
//!
//! - Token price fetcher, which updates prices for all the tokens we use in micro.
//!   Data of this fetcher is used to calculate fees.
//! - Token trading volume fetcher, which updates trading volumes for tokens.
//!   Data of this fetcher is used to decide whether we are going to accept fees in this token.
//!
//! Every data fetcher is represented by an autonomic routine, which spend most of the time sleeping;
//! once in the configurable interval it fetches the data from an API and store it into the database.

use micro_config::MicroConfig;
use micro_dal::ConnectionPool;
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub mod error;
pub mod token_list;
pub mod token_price;
pub mod token_trading_volume;

pub fn run_data_fetchers(
    config: &MicroConfig,
    pool: ConnectionPool,
    stop_receiver: watch::Receiver<bool>,
) -> Vec<JoinHandle<()>> {
    let list_fetcher = token_list::TokenListFetcher::new(config.clone());
    let price_fetcher = token_price::TokenPriceFetcher::new(config.clone());
    let volume_fetcher = token_trading_volume::TradingVolumeFetcher::new(config.clone());

    vec![
        tokio::spawn(list_fetcher.run(pool.clone(), stop_receiver.clone())),
        tokio::spawn(price_fetcher.run(pool.clone(), stop_receiver.clone())),
        tokio::spawn(volume_fetcher.run(pool, stop_receiver)),
    ]
}
