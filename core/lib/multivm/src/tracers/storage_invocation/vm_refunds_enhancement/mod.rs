use micro_state::WriteStorage;

use crate::{
    interface::{
        tracer::{TracerExecutionStatus, TracerExecutionStopReason},
        traits::tracers::dyn_tracers::vm_1_3_3::DynTracer,
        Halt,
    },
    tracers::storage_invocation::StorageInvocations,
    vm_refunds_enhancement::{BootloaderState, HistoryMode, MicroVmState, SimpleMemory, VmTracer},
};

impl<S, H: HistoryMode> DynTracer<S, SimpleMemory<H>> for StorageInvocations {}

impl<S: WriteStorage, H: HistoryMode> VmTracer<S, H> for StorageInvocations {
    fn finish_cycle(
        &mut self,
        state: &mut MicroVmState<S, H>,
        _bootloader_state: &mut BootloaderState,
    ) -> TracerExecutionStatus {
        let current = state
            .storage
            .storage
            .get_ptr()
            .borrow()
            .missed_storage_invocations();

        if current >= self.limit {
            return TracerExecutionStatus::Stop(TracerExecutionStopReason::Abort(
                Halt::TracerCustom("Storage invocations limit reached".to_string()),
            ));
        }
        TracerExecutionStatus::Continue
    }
}
