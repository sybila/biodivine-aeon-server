use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use std::sync::Mutex;

impl ProgressTracker {

    pub fn new(graph: &AsyncGraph) -> ProgressTracker {
        let unit_cardinality = graph.unit_params().cardinality();
        let num_states = graph.num_states() as f64;
        let graph_size = unit_cardinality * num_states;
        return ProgressTracker {
            total: graph_size,
            remaining: Mutex::new(graph_size)
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

}