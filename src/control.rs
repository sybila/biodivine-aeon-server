use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use biodivine_pbn_control::perturbation::PerturbationGraph;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ControlComputation {
    pub timestamp: SystemTime,
    pub finished_timestamp: Option<SystemTime>,
    pub input_model: String,
    pub graph: Arc<PerturbationGraph>,
    pub thread: Option<JoinHandle<()>>,
    pub results: Option<Vec<(HashMap<String, bool>, GraphColors)>>,
    pub is_cancelled: cancel_this::CancelAtomic,
}

impl ControlComputation {
    pub fn start_timestamp(&self) -> u128 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .expect("Time error")
            .as_millis()
    }

    pub fn end_timestamp(&self) -> Option<u128> {
        self.finished_timestamp.map(|t| {
            t.duration_since(UNIX_EPOCH)
                .expect("Time error")
                .as_millis()
        })
    }
}
