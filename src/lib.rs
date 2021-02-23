#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate json;
extern crate html_escape;
extern crate tinyfiledialogs as tfd;

use crate::requests::process_request;
use crate::scc::{Classifier, ProgressTracker};
use biodivine_lib_param_bn::async_graph::{AsyncGraph, DefaultEdgeParams};
use json::JsonValue;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};
use tfd::{MessageBoxIcon, YesNo};
use web_view::{Content, WVResult, WebView};

pub mod scc;

/// Module for handling different types of Aeon API requests.
pub mod requests;

const INDEX: &str = include_str!("../assets/index.bundle.html");

lazy_static! {
    /// Used to open new windows. The string contains aeon model which is supposed to be open.
    pub static ref OPEN_MODEL: Mutex<Option<Sender<String>>> = Mutex::new(None);
    //pub static ref COMPUTATION: Arc<RwLock<Option<Computation>>> = Arc::new(RwLock::new(None));
}

// Global state of AEON session:

type ArcComputation = Arc<RwLock<Option<Computation>>>;

pub struct AeonSession {
    window: WebView<'static, ArcComputation>,
}

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

impl AeonSession {
    fn new_with_html(html: &str) -> AeonSession {
        let cmp: ArcComputation = Arc::new(RwLock::new(None));

        let web_view = web_view::builder()
            .title("Aeon 2019")
            .content(Content::Html(html))
            .size(1200, 800)
            .resizable(true)
            //.debug(true)
            .user_data(cmp)
            .invoke_handler(|web_view, arg| handle_bridge_request(web_view, arg))
            .build()
            .unwrap();
        AeonSession { window: web_view }
    }

    pub fn new() -> AeonSession {
        Self::new_with_html(INDEX)
    }

    pub fn new_with_model(model: &str) -> AeonSession {
        let model = html_escape::encode_double_quoted_attribute(model);
        let index_str = INDEX.replace(
            "data-initial-model=\"undefined\"",
            &format!("data-initial-model=\"{}\"", model),
        );
        Self::new_with_html(&index_str)
    }

    pub fn step(&mut self) -> Option<WVResult> {
        self.window.step()
    }
}

pub fn handle_bridge_request(web_view: &mut WebView<ArcComputation>, arg: &str) -> WVResult {
    println!("Request: {}", arg);
    match json::parse(arg) {
        Ok(json) => {
            let id: u64 = json["requestId"].as_u64().unwrap();
            let path: &str = json["path"].as_str().unwrap();
            if path == "log" {
                println!("{}", json["message"]);
                Ok(())
            } else if path == "alert" {
                let message: String = json["message"].as_str().map(|i| i.to_string()).unwrap_or(String::new());
                let message_type: &str = json["type"].as_str().unwrap();
                let icon = match message_type {
                    "error" => MessageBoxIcon::Error,
                    "warning" => MessageBoxIcon::Warning,
                    "question" => MessageBoxIcon::Question,
                    _ => MessageBoxIcon::Info
                };
                tfd::message_box_ok("Aeon", &message, icon);
                Ok(())
            } else if path == "confirm" {
                println!("Confirm");
                let message: &str = json["message"].as_str().unwrap();
                let result = match tfd::message_box_yes_no("Aeon", message, MessageBoxIcon::Warning, YesNo::No) {
                    YesNo::No => false,
                    YesNo::Yes => true
                };
                let command = format!("NativeBridge.handleResponse({})", json::stringify(object! {
                    "status": true,
                    "result": object! {
                        "yes": result,
                    },
                    "requestId": id,
                }));
                web_view.eval(&command)
            } else if path == "save_file" {
                let suggested_name: &str = json["name"].as_str().unwrap();
                let content: &str = json["content"].as_str().unwrap();
                match tfd::save_file_dialog("Save file...", suggested_name) {
                    Some(path) => match std::fs::write(&path, &content) {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            tfd::message_box_ok(
                                "File Error",
                                &format!("Cannot save file: {}", e),
                                MessageBoxIcon::Error,
                            );
                            Ok(())
                        }
                    },
                    None => Ok(()),
                }
            } else {
                let mut response =
                    process_request(web_view.user_data().clone(), path, &json).json();
                response["requestId"] = id.into();
                let command = format!("NativeBridge.handleResponse({})", json::stringify(response));
                println!("Command: {}", command);
                web_view.eval(&command)
            }
        }
        Err(e) => panic!("Invalid native request: {:?}", e),
    }
}

pub struct BackendResponse {
    pub response: JsonValue,
}

impl BackendResponse {
    pub fn ok_json(data: JsonValue) -> Self {
        return BackendResponse {
            response: object! {
                "status": true,
                "result": data,
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
