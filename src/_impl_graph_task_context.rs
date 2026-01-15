use crate::GraphTaskContext;
use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use cancel_this::CancellationTrigger;

impl Default for GraphTaskContext {
    fn default() -> Self {
        GraphTaskContext::new()
    }
}

impl GraphTaskContext {
    /// Create a new task context.
    pub fn new() -> GraphTaskContext {
        GraphTaskContext {
            is_cancelled: cancel_this::CancelAtomic::new(),
            progress: ProgressTracker::new(),
        }
    }

    pub fn init_progress(&self, graph: &SymbolicAsyncGraph) {
        self.progress.init_from_graph(graph);
    }

    /// True if the task is canceled.
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.is_cancelled()
    }

    /// Set the status of this task to cancel.
    pub fn cancel(&self) {
        self.is_cancelled.cancel();
    }

    /// Indicate that the given set still needs to be processed by the task.
    pub fn update_remaining(&self, remaining: &GraphColoredVertices) {
        self.progress.update_remaining(remaining);
    }

    pub fn increment_result_count(&self) {
        self.progress.increment_result_count();
    }

    /// Output a string that represents the percentage of remaining state space.
    pub fn get_progress_string(&self) -> String {
        self.progress.get_progress_string()
    }
}
