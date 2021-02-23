#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate json;
extern crate html_escape;
extern crate native_dialog;

use crate::requests::process_request;
use crate::scc::{Classifier, ProgressTracker};
use biodivine_lib_param_bn::async_graph::{AsyncGraph, DefaultEdgeParams};
use json::JsonValue;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};
use web_view::{Content, WVResult, WebView};
use native_dialog::{MessageDialog, MessageType, FileDialog};

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
    // TODO: This print should be enabled/disabled based on some env. property.
    //println!("Request: {}", arg);
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
                    "error" => MessageType::Error,
                    "warning" => MessageType::Warning,
                    "question" => MessageType::Warning,
                    _ => MessageType::Info
                };
                MessageDialog::new()
                    .set_type(icon)
                    .set_text(&message)
                    .set_title("Aeon")
                    .show_alert()
                    .unwrap();
                Ok(())
            } else if path == "confirm" {
                let message: &str = json["message"].as_str().unwrap();
                let result = MessageDialog::new()
                    .set_text(message)
                    .set_title("Aeon")
                    .show_confirm()
                    .unwrap();
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
                let path = FileDialog::new()
                    .set_location("~")
                    .set_filename(suggested_name)
                    .show_save_single_file()
                    .unwrap();
                if let Some(path) = path {
                    // On MacOS this sometimes happens...
                    let path = path.display().to_string();
                    let path = if path.starts_with("file://") {
                        &path[7..]
                    } else {
                        &path
                    };
                    match std::fs::write(&path, &content) {
                        Ok(()) => (),
                        Err(e) => {
                            MessageDialog::new()
                                .set_type(MessageType::Error)
                                .set_title("File Error")
                                .set_text(&format!("Cannot save file: {}", e))
                                .show_alert()
                                .unwrap();
                        }
                    };
                }
                Ok(())
            } else {
                let mut response =
                    process_request(web_view.user_data().clone(), path, &json).json();
                response["requestId"] = id.into();
                let command = format!("NativeBridge.handleResponse({})", json::stringify(response));
                //println!("Command: {}", command);
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
