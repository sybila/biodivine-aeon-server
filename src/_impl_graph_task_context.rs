use crate::scc::ProgressTracker;
use crate::GraphTaskContext;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use std::sync::atomic::{AtomicBool, Ordering};

impl GraphTaskContext {
    /// Create a new task context.
    pub fn new(graph: &SymbolicAsyncGraph) -> GraphTaskContext {
        GraphTaskContext {
            is_cancelled: AtomicBool::new(false),
            progress: ProgressTracker::new(graph),
        }
    }

    /// True if the task is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    /// Set the status of this task to cancelled.
    pub fn cancel(&mut self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    /// Indicate that the given set still needs to be processed by the task.
    pub fn update_remaining(&self, remaining: &GraphColoredVertices) {
        self.progress
            .update_remaining(remaining.approx_cardinality());
    }
}
