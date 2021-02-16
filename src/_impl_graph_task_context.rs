use crate::scc::ProgressTracker;
use crate::GraphTaskContext;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use std::sync::atomic::{AtomicBool, Ordering};

impl GraphTaskContext {
    /// Create a new task context.
    pub fn new() -> GraphTaskContext {
        GraphTaskContext {
            is_cancelled: AtomicBool::new(false),
            progress: ProgressTracker::new(),
        }
    }

    /// Re-initialize the task context with a fresh graph.
    pub fn restart(&self, graph: &SymbolicAsyncGraph) {
        self.progress.init_from_graph(graph);
        self.is_cancelled.store(false, Ordering::SeqCst);
    }

    /// True if the task is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    /// Set the status of this task to cancelled.
    ///
    /// Return true if the computation was set to cancelled by this call, false if it was
    /// cancelled previously.
    pub fn cancel(&self) -> bool {
        !self.is_cancelled.swap(true, Ordering::SeqCst)
    }

    /// Indicate that the given set still needs to be processed by the task.
    pub fn update_remaining(&self, remaining: &GraphColoredVertices) {
        self.progress.update_remaining(remaining);
    }

    /// Output a string which represent the percentage of remaining state space.
    pub fn get_percent_string(&self) -> String {
        self.progress.get_percent_string()
    }
}
