use crate::scc::ProgressTracker;
use bigdecimal::BigDecimal;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use num_bigint::BigUint;
use num_traits::Zero;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressTracker {
    /// Create a new uninitialized progress counter.
    pub fn new() -> ProgressTracker {
        ProgressTracker {
            total: Mutex::new(BigUint::zero()),
            remaining: Mutex::new(BigUint::zero()),
            results: AtomicUsize::default(),
        }
    }

    /// Restart progress counter with given graph.
    pub fn init_from_graph(&self, graph: &SymbolicAsyncGraph) {
        let all_states = graph.unit_colored_vertices().exact_cardinality();
        let mut total = self.total.lock().unwrap();
        *total = all_states.clone();
        let mut remaining = self.remaining.lock().unwrap();
        *remaining = all_states;
    }

    /// Update the number of remaining states.
    pub fn update_remaining(&self, remaining: &GraphColoredVertices) {
        let value = remaining.exact_cardinality();
        let mut changed = false;
        {
            let mut remaining = self.remaining.lock().unwrap();
            if *remaining != value {
                *remaining = value;
                changed = true;
            }
        }
        if changed {
            println!("Progress: {}", self.get_progress_string());
        }
    }

    pub fn increment_result_count(&self) {
        self.results.fetch_add(1, Ordering::SeqCst);
    }

    /// Output a string that represents the percentage of remaining state space.
    pub fn get_progress_string(&self) -> String {
        let remaining = BigDecimal::from_biguint(self.remaining.lock().unwrap().clone(), 0);
        let total = BigDecimal::from_biguint(self.total.lock().unwrap().clone(), 0);
        let attractors = self.results.load(Ordering::SeqCst);
        if attractors > 0 {
            format!("{remaining:.4e}/{total:.4e} ({attractors} attractors)")
        } else {
            format!("{remaining:.4e}/{total:.4e}")
        }
    }
}
