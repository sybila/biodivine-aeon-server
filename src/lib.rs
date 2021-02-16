use crate::scc::ProgressTracker;
use std::sync::atomic::AtomicBool;

mod _impl_graph_task_context;
pub mod scc;

/// A context object which aggregates all necessary information about a running task working with
/// a symbolic graph.
///
/// We use this to avoid passing each context variable as a (mutable) reference. It is also easier
/// to implement some utility methods this way.
pub struct GraphTaskContext {
    is_cancelled: AtomicBool,
    progress: ProgressTracker,
}
