use micro_state::WriteStorage;
use micro_types::vm_trace::Call;
use multivm::MultivmTracer;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use vm::{CallTracer, HistoryMode};

/// Custom tracers supported by our api
#[derive(Debug)]
pub(crate) enum ApiTracer {
    CallTracer(Arc<OnceCell<Vec<Call>>>),
}

impl ApiTracer {
    pub fn into_boxed<
        S: WriteStorage,
        H: HistoryMode + multivm::HistoryMode<VmVirtualBlocksRefundsEnhancement = H> + 'static,
    >(
        self,
    ) -> Box<dyn MultivmTracer<S, H>> {
        match self {
            ApiTracer::CallTracer(tracer) => CallTracer::new(tracer, H::default()).into_boxed(),
        }
    }
}
