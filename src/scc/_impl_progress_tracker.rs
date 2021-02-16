use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

impl ProgressTracker {
    /// Create a new uninitialized progress counter.
    pub fn new() -> ProgressTracker {
        ProgressTracker {
            processes: AtomicU32::new(0),
            total: Mutex::new(0.0),
            remaining: Mutex::new(0.0),
        }
    }

    /// Restart progress counter with given graph.
    pub fn init_from_graph(&self, graph: &SymbolicAsyncGraph) {
        let all_states = graph.unit_vertices().approx_cardinality();
        let mut total = self.total.lock().unwrap();
        *total = all_states;
        let mut remaining = self.remaining.lock().unwrap();
        *remaining = all_states;
    }

    /// Set number of running processes.
    pub fn set_process_count(&self, count: u32) {
        self.processes.store(count, Ordering::SeqCst);
    }

    /// Subtract one from the number of running processes.
    pub fn process_finished(&self) {
        self.processes.fetch_sub(1, Ordering::SeqCst);
        println!("Progress: {}", self.get_percent_string());
    }

    /// Update the number of remaining states.
    pub fn update_remaining(&self, remaining: &GraphColoredVertices) {
        let value = remaining.approx_cardinality();
        {
            let mut remaining = self.remaining.lock().unwrap();
            *remaining = value;
        }
        println!("Progress: {}", self.get_percent_string());
    }

    /// Return a `[0,1]` fraction of the remaining state space.
    pub fn get_remaining_fraction(&self) -> f64 {
        let remaining = { *self.remaining.lock().unwrap() };
        let total = { *self.total.lock().unwrap() };
        return remaining / total;
    }

    /// Return a `[0,1]` fraction of the remaining log-state-space. This metric better
    /// corresponds to the percentage of the "model" processed.
    pub fn get_remaining_log_fraction(&self) -> f64 {
        let remaining = { *self.remaining.lock().unwrap() };
        let total = { *self.total.lock().unwrap() };
        return remaining.log2() / total.log2();
    }

    /// Output a string which represent the percentage of remaining state space.
    pub fn get_percent_string(&self) -> String {
        format!(
            "{:.2}% ({:.2}% states, {} processes)",
            100.0 - (self.get_remaining_log_fraction() * 100.0),
            100.0 - (self.get_remaining_fraction() * 100.0),
            self.processes.load(Ordering::SeqCst),
        )
    }
}
