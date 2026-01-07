use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use biodivine_pbn_control::perturbation::PerturbationGraph;
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ControlComputation {
    pub timestamp: SystemTime,
    pub finished_timestamp: Option<SystemTime>,
    pub input_model: String,
    pub graph: PerturbationGraph,
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

    pub fn elapsed_millis(&self) -> u64 {
        let elapsed = self
            .finished_timestamp
            .unwrap_or_else(SystemTime::now)
            .duration_since(self.timestamp)
            .unwrap();
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
    }
}
