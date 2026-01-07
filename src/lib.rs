#[macro_use]
extern crate json;

use crate::scc::ProgressTracker;

/// Contains all non-trivial long-running symbolic algorithms that are used within AEON.
pub mod algorithms;

pub mod bdt;
pub mod control;
pub mod scc;
/// Some utility methods which we can later move to std-lib
pub mod util;

mod _impl_graph_task_context;

/// A context object which aggregates all necessary information about a running task working with
/// a symbolic graph.
///
/// We use this to avoid passing each context variable as a (mutable) reference. It is also easier
/// to implement some utility methods this way.
pub struct GraphTaskContext {
    pub is_cancelled: cancel_this::CancelAtomic,
    progress: ProgressTracker,
}
