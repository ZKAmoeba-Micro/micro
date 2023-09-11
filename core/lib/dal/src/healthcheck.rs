use crate::ConnectionPool;
use micro_health_check::{CheckHealth, CheckHealthStatus};

// HealthCheck used to verify if we can connect to the database.
// This guarantees that the app can use it's main "communication" channel.
// Used in the /health endpoint
#[derive(Clone, Debug)]
pub struct ConnectionPoolHealthCheck {
    connection_pool: ConnectionPool,
}

impl ConnectionPoolHealthCheck {
    pub fn new(connection_pool: ConnectionPool) -> ConnectionPoolHealthCheck {
        Self { connection_pool }
    }
}

impl CheckHealth for ConnectionPoolHealthCheck {
    fn check_health(&self) -> CheckHealthStatus {
        // This check is rather feeble, plan to make reliable here:
        // https://linear.app/matterlabs/issue/PLA-255/revamp-db-connection-health-check
        let _ = self.connection_pool.access_storage_blocking();
        CheckHealthStatus::Ready
    }
}
