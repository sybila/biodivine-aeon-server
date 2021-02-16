use std::sync::atomic::AtomicBool;
use crate::scc::ProgressTracker;

pub mod scc;
mod _impl_task_context;

pub struct TaskContext {
    is_cancelled: AtomicBool,
    progress: ProgressTracker,
}