use micro_config::configs::ProverGroupConfig;
use micro_dal::ConnectionPool;
use micro_prover_utils::circuit_name_to_numeric_index;

use crate::house_keeper::periodic_job::PeriodicJob;

#[derive(Debug)]
pub struct ProverStatsReporter {
    reporting_interval_ms: u64,
}

impl ProverStatsReporter {
    pub fn new(reporting_interval_ms: u64) -> Self {
        Self {
            reporting_interval_ms,
        }
    }
}

/// Invoked periodically to push job statistics to Prometheus
/// Note: these values will be used for manually scaling provers.
impl PeriodicJob for ProverStatsReporter {
    const SERVICE_NAME: &'static str = "ProverStatsReporter";

    fn run_routine_task(&mut self, connection_pool: ConnectionPool) {
        let prover_group_config = ProverGroupConfig::from_env();
        let mut conn = connection_pool.access_storage_blocking();
        let stats = conn.prover_dal().get_prover_jobs_stats_per_circuit();

        for (circuit_name, stats) in stats.into_iter() {
            let group_id = prover_group_config
                .get_group_id_for_circuit_id(circuit_name_to_numeric_index(&circuit_name).unwrap())
                .unwrap();

            metrics::gauge!(
              "server.prover.jobs",
              stats.queued as f64,
              "type" => "queued",
              "prover_group_id" => group_id.to_string(),
              "circuit_name" => circuit_name.clone(),
              "circuit_type" => circuit_name_to_numeric_index(&circuit_name).unwrap().to_string()
            );

            metrics::gauge!(
              "server.prover.jobs",
              stats.in_progress as f64,
              "type" => "in_progress",
              "prover_group_id" => group_id.to_string(),
              "circuit_name" => circuit_name.clone(),
              "circuit_type" => circuit_name_to_numeric_index(&circuit_name).unwrap().to_string()
            );
        }

        if let Some(min_unproved_l1_batch_number) = conn.prover_dal().min_unproved_l1_batch_number()
        {
            metrics::gauge!("server.block_number", min_unproved_l1_batch_number.0 as f64, "stage" => "circuit_aggregation")
        }

        let lag_by_circuit_type = conn
            .prover_dal()
            .min_unproved_l1_batch_number_by_basic_circuit_type();

        for (circuit_type, l1_batch_number) in lag_by_circuit_type {
            metrics::gauge!("server.block_number", l1_batch_number.0 as f64, "stage" => format!("circuit_{}", circuit_type));
        }
    }

    fn polling_interval_ms(&self) -> u64 {
        self.reporting_interval_ms
    }
}
