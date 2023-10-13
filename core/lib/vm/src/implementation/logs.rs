use zk_evm::aux_structures::Timestamp;
use micro_state::WriteStorage;

use micro_types::event::extract_l2tol1logs_from_l1_messenger;
use micro_types::l2_to_l1_log::L2ToL1Log;
use micro_types::VmEvent;

use crate::old_vm::history_recorder::HistoryMode;
use crate::old_vm::utils::precompile_calls_count_after_timestamp;
use crate::types::outputs::VmExecutionLogs;
use crate::utils::logs;
use crate::vm::Vm;

impl<S: WriteStorage, H: HistoryMode> Vm<S, H> {
    pub(crate) fn collect_execution_logs_after_timestamp(
        &self,
        from_timestamp: Timestamp,
    ) -> VmExecutionLogs {
        let storage_logs: Vec<_> = self
            .state
            .storage
            .storage_log_queries_after_timestamp(from_timestamp)
            .iter()
            .map(|log| **log)
            .collect();
        let storage_logs_count = storage_logs.len();

        let (events, system_l2_to_l1_logs) =
            self.collect_events_and_l1_logs_after_timestamp(from_timestamp);

        let log_queries = self
            .state
            .event_sink
            .log_queries_after_timestamp(from_timestamp);

        let precompile_calls_count = precompile_calls_count_after_timestamp(
            self.state.precompiles_processor.timestamp_history.inner(),
            from_timestamp,
        );

        let user_logs = extract_l2tol1logs_from_l1_messenger(&events);

        let total_log_queries_count =
            storage_logs_count + log_queries.len() + precompile_calls_count;

        VmExecutionLogs {
            storage_logs,
            events,
            user_l2_to_l1_logs: user_logs.into_iter().map(|log| log.into()).collect(),
            system_l2_to_l1_logs,
            total_log_queries_count,
        }
    }

    pub(crate) fn collect_events_and_l1_logs_after_timestamp(
        &self,
        from_timestamp: Timestamp,
    ) -> (Vec<VmEvent>, Vec<L2ToL1Log>) {
        logs::collect_events_and_l1_logs_after_timestamp(
            &self.state,
            &self.batch_env,
            from_timestamp,
        )
    }
}
