pub mod error;
pub mod ethereum;
pub mod operations;
pub mod signer;
pub mod utils;
pub mod wallet;

pub use crate::{ethereum::EthereumProvider, wallet::Wallet};
pub use micro_types::network::Network;

pub use micro_types;
pub use micro_types::web3;

pub use micro_web3_decl::{
    jsonrpsee::http_client::*,
    namespaces::{EthNamespaceClient, NetNamespaceClient, Web3NamespaceClient, ZksNamespaceClient},
    types,
};
