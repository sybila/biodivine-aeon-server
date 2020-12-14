use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use std::sync::Mutex;

impl ProgressTracker {
    pub fn new(graph: &SymbolicAsyncGraph) -> ProgressTracker {
        let unit_cardinality = graph.unit_colors().approx_cardinality();
        let num_states = graph.unit_vertices().vertices().approx_cardinality();
        let graph_size = unit_cardinality * num_states;
        return ProgressTracker {
            total: graph_size,
            remaining: Mutex::new(graph_size),
            last_wave: Mutex::new(0.0),
        };
    }

    pub fn update_last_wave(&self, value: f64) {
        {
            let mut last_wave = self.last_wave.lock().unwrap();
            *last_wave = value;
        }
    }

    pub fn update_remaining(&self, value: f64) {
        {
            let mut remaining = self.remaining.lock().unwrap();
            *remaining = value;
        }
        println!("Progress: {:.2}%", 100.0 - (self.get_progress() * 100.0));
    }

    // return the % (0.0 - 1.0 value) of state space that remains to be processed
    pub fn get_progress(&self) -> f64 {
        let remaining = { *self.remaining.lock().unwrap() };
        return remaining / self.total;
    }

    pub fn get_percent_string(&self) -> String {
        return format!(
            "{:.2}% (Reachability remaining: {})",
            100.0 - (self.get_progress() * 100.0),
            { *self.last_wave.lock().unwrap() }
        );
    }
}
