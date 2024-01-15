use micro_config::configs::fri_prover_dashboard::FriProverDashboardConfig;

use crate::{envy_load, FromEnv};

impl FromEnv for FriProverDashboardConfig {
    fn from_env() -> anyhow::Result<Self> {
        envy_load("fri_prover_dashboard", "FRI_PROVER_DASHBOARD_")
    }
}
