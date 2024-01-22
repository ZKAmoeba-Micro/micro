use micro_dal::SqlxError;
use micro_eth_client::types::Error as MicroClientError;

#[derive(Debug, thiserror::Error)]
pub enum TaskApplyError {
    #[error("Eth client error: {0}")]
    MicroClient(#[from] MicroClientError),
    #[error("Database error :{0}")]
    DatabaseError(#[from] SqlxError),
    #[error("ClientError error :{0}")]
    ClientError(String),
    #[error("WalletError error :{0}")]
    WalletError(String),
}
