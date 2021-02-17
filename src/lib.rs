#[macro_use]
extern crate json;

use crate::scc::ProgressTracker;
use std::sync::atomic::AtomicBool;

pub mod all;
pub mod bdt;
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
    is_cancelled: AtomicBool,
    progress: ProgressTracker,
}
