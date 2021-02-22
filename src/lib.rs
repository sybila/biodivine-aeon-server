#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate json;

use crate::scc::{Classifier, ProgressTracker};
use biodivine_lib_param_bn::async_graph::{AsyncGraph, DefaultEdgeParams};
use json::JsonValue;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod scc;

/// Module for handling different types of Aeon API requests.
pub mod requests;

// Global state of AEON session:

/// Computation keeps all information
pub struct Computation {
    pub timestamp: SystemTime,
    pub is_cancelled: Arc<AtomicBool>, // indicate to the server that the computation should be cancelled
    pub input_model: String,           // .aeon string representation of the model
    pub graph: Option<Arc<AsyncGraph<DefaultEdgeParams>>>, // Model graph - used to create witnesses
    pub classifier: Option<Arc<Classifier>>, // Classifier used to store the results of the computation
    pub progress: Option<Arc<ProgressTracker>>, // Used to access progress of the computation
    pub thread: Option<JoinHandle<()>>, // A thread that is actually doing the computation (so that we can check if it is still running). If none, the computation is done.
    pub error: Option<String>,          // A string error from the computation
    pub finished_timestamp: Option<SystemTime>, // A timestamp when the computation was completed (if done)
}

impl Computation {
    pub fn start_timestamp(&self) -> u128 {
        return self
            .timestamp
            .duration_since(UNIX_EPOCH)
            .expect("Time error")
            .as_millis();
    }

    pub fn end_timestamp(&self) -> Option<u128> {
        return self.finished_timestamp.map(|t| {
            t.duration_since(UNIX_EPOCH)
                .expect("Time error")
                .as_millis()
        });
    }
}

lazy_static! {
    pub static ref COMPUTATION: Arc<RwLock<Option<Computation>>> = Arc::new(RwLock::new(None));
}

pub struct BackendResponse {
    pub response: JsonValue,
}

impl BackendResponse {
    pub fn ok_json(json: JsonValue) -> Self {
        return BackendResponse {
            response: object! {
                "status": true,
                "result": json,
            },
        };
    }

    pub fn ok(message: &String) -> Self {
        return BackendResponse {
            response: object! {
                "status": true,
                "result": json::parse(message).unwrap(),
            }, //message: format!("{{ \"status\": true, \"result\": {} }}", message),
        };
    }

    pub fn err(message: &String) -> Self {
        return BackendResponse {
            response: object! {
                "status": false,
                "message": message.clone(),
            }, //message: format!("{{ \"status\": false, \"message\": \"{}\" }}", message),
        };
    }

    pub fn json(self) -> JsonValue {
        self.response
    }
}