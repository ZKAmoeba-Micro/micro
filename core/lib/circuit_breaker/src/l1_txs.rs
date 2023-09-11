use crate::{CircuitBreaker, CircuitBreakerError};
use micro_dal::ConnectionPool;

#[derive(Debug)]
pub struct FailedL1TransactionChecker {
    pub pool: ConnectionPool,
}

#[async_trait::async_trait]
impl CircuitBreaker for FailedL1TransactionChecker {
    async fn check(&self) -> Result<(), CircuitBreakerError> {
        if self
            .pool
            .access_storage_blocking()
            .eth_sender_dal()
            .get_number_of_failed_transactions()
            > 0
        {
            return Err(CircuitBreakerError::FailedL1Transaction);
        }
        Ok(())
    }
}
