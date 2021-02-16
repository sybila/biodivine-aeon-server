use crate::TaskContext;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;

impl TaskContext {

    pub fn new(graph: &SymbolicAsyncGraph) -> TaskContext {
        TaskContext {
            is_cancelled: AtomicBool::new(false),
            progress: ProgressTracker::new(graph),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

}