use biodivine_pbn_control::control::PhenotypeControlMap;
use biodivine_pbn_control::perturbation::PerturbationGraph;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ControlComputation {
    pub timestamp: SystemTime,
    pub finished_timestamp: Option<SystemTime>,
    pub input_model: String,
    pub graph: Arc<PerturbationGraph>,
    pub thread: Option<JoinHandle<()>>,
    pub results: Option<PhenotypeControlMap>,
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
