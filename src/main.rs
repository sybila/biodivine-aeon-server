#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate json;

use rocket::http::hyper::header::AccessControlAllowOrigin;
use rocket::http::{ContentType, Header};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use crate::scc::{Behaviour, Class, Classifier, ProgressTracker};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use regex::Regex;
use std::convert::TryFrom;

pub mod scc;
mod test_main;

use crate::scc::algo_components::components;
use rocket::Data;
use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Computation keeps all information
struct Computation {
    timestamp: SystemTime,
    is_cancelled: Arc<AtomicBool>, // indicate to the server that the computation should be cancelled
    input_model: String,           // .aeon string representation of the model
    graph: Option<Arc<AsyncGraph>>, // Model graph - used to create witnesses
    classifier: Option<Arc<Classifier>>, // Classifier used to store the results of the computation
    progress: Option<Arc<ProgressTracker>>, // Used to access progress of the computation
    thread: Option<JoinHandle<()>>, // A thread that is actually doing the computation (so that we can check if it is still running). If none, the computation is done.
    error: Option<String>,          // A string error from the computation
}

impl Computation {
    pub fn start_timestamp(&self) -> u128 {
        return self.timestamp.duration_since(UNIX_EPOCH)
            .expect("Time error")
            .as_millis();
    }
}

lazy_static! {
    static ref COMPUTATION: Arc<RwLock<Option<Computation>>> = Arc::new(RwLock::new(None));
}

/// Accept a partial model containing only the necessary regulations and one update function.
/// Return cardinality of such model (i.e. the number of instantiations of this update function)
/// or error if the update function (or model) is invalid.
/// TODO: On some large models, this sometimes returns some bogus number even though the model is too large to run :O
#[post("/check_update_function", format = "plain", data = "<data>")]
fn check_update_function(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model size to 10MB
    let mut model_string = String::new();
    return match stream.read_to_string(&mut model_string) {
        Ok(_) => {
            let graph = BooleanNetwork::try_from(model_string.as_str()).and_then(|model| {
                if model.graph().num_vars() <= 5 {
                    AsyncGraph::new(model)
                } else {
                    Err("Function too large for on-the-fly analysis.".to_string())
                }
            });
            match graph {
                Ok(graph) => BackendResponse::ok(&format!(
                    "{{\"cardinality\":\"{}\"}}",
                    graph.unit_params().cardinality()
                )),
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

#[get("/ping")]
fn ping() -> BackendResponse {
    println!("...ping...");
    let mut response = object! {
        "timestamp" => json::Null,          // if there is some computation (not necessarily running, this is the time when it started
        "is_cancelled" => false,            // true if the computation has been canceled
        "running" => false,                 // true if the computation thread is still alive
        "progress" => "unknown".to_string(),// arbitrary progress string
        "error" => json::Null,              // arbitrary error string - currently not really used
        "num_classes" => json::Null         // number of discovered classes so far
    };
    {
        // Read data from current computation if available...
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(computation) = &*cmp {
            response["timestamp"] = (computation.start_timestamp() as u64).into();
            response["is_cancelled"] = computation.is_cancelled.load(Ordering::SeqCst).into();
            if let Some(progress) = &computation.progress {
                response["progress"] = progress.get_percent_string().into();
            }
            response["is_running"] = computation.thread.is_some().into();
            if let Some(error) = &computation.error {
                response["error"] = error.clone().into();
            }
            if let Some(classes) = computation.classifier.as_ref().map(|c| c.try_get_num_classes()) {
                response["num_classes"] = classes.into();
            }
        }
    }
    return BackendResponse::ok(&response.to_string());
}

#[get("/get_results")]
fn get_results() -> BackendResponse {
    let is_partial;
    let data = {
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            is_partial = cmp.thread.is_some();
            if let Some(classes) = &cmp.classifier {
                let mut result = None;
                for _ in 0..5 {
                    if let Some(data) = classes.try_export_result() {
                        result = Some(data);
                        break;
                    }
                    // wait a little - maybe the lock will become free
                    std::thread::sleep(Duration::new(1, 0));
                }
                if let Some(result) = result {
                    result
                } else {
                    return BackendResponse::err(&"Classification running. Cannot export components right now.".to_string());
                }
            } else {
                return BackendResponse::err(&"Results not available yet.".to_string());
            }
        } else {
            return BackendResponse::err(&"No results available.".to_string());
        }
    };
    let lines: Vec<String> = data
        .iter()
        .map(|(c, p)| format!("{{\"sat_count\":{},\"phenotype\":{}}}", p.cardinality(), c))
        .collect();

    println!("Result {:?}", lines);

    let mut json = String::new();
    for index in 0..lines.len() - 1 {
        json += &format!("{},", lines[index]);
    }
    json = format!(
        "{{ \"isPartial\":{}, \"data\":[{}{}] }}",
        is_partial,
        json,
        lines.last().unwrap()
    );

    return BackendResponse::ok(&json);
}

#[get("/get_witness/<class_str>")]
fn get_witness(class_str: String) -> BackendResponse {
    let mut class = Class::new_empty();
    for char in class_str.chars() {
        match char {
            'D' => class.extend(Behaviour::Disorder),
            'O' => class.extend(Behaviour::Oscillation),
            'S' => class.extend(Behaviour::Stability),
            _ => {
                return BackendResponse::err(&"Invalid class.".to_string());
            }
        }
    }
    {
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if let Some(classifier) = &cmp.classifier {
                if let Some(class) = classifier.get_params(&class) {
                    if let Some(graph) = &cmp.graph {
                        let witness = graph.make_witness(&class);
                        let layout = read_layout(cmp.input_model.as_str());
                        let mut model_string = format!("{}", witness); // convert back to aeon
                        model_string += "\n";
                        for (var, (x, y)) in layout {
                            model_string += format!("#position:{}:{},{}\n", var, x, y).as_str();
                        }
                        let (name, description) = read_metadata(cmp.input_model.as_str());
                        if let Some(name) = name {
                            model_string += format!("#name:{}\n", name).as_str();
                        }
                        if let Some(description) = description {
                            model_string += format!("#description:{}\n", description).as_str();
                        }
                        BackendResponse::ok(&object! { "model" => model_string }.to_string())
                    } else {
                        return BackendResponse::err(&"No results available.".to_string());
                    }
                } else {
                    return BackendResponse::err(&"Specified class has no witness.".to_string());
                }
            } else {
                return BackendResponse::err(&"No results available.".to_string());
            }
        } else {
            return BackendResponse::err(&"No results available.".to_string());
        }
    }
}

/// Accept an Aeon model, parse it and start a new computation (if there is no computation running).
#[post("/start_computation", format = "plain", data = "<data>")]
fn start_computation(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => {
            // First, try to parse the network so that the user can at least verify it is correct...
            match BooleanNetwork::try_from(aeon_string.as_str()) {
                Ok(network) => {
                    // Now we can try to start the computation...
                    let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
                    {
                        // First, just try to read the computation, if there is something
                        // there, we just want to quit fast...
                        let cmp = cmp.read().unwrap();
                        if let Some(computation) = &*cmp {
                            if computation.thread.is_some() {
                                return BackendResponse::err(&"Previous computation is still running. Cancel it before starting a new one.".to_string());
                            }
                        }
                    }
                    {
                        // Now actually get the write lock, but check again because race conditions...
                        let mut cmp = cmp.write().unwrap();
                        if let Some(computation) = &*cmp {
                            if computation.thread.is_some() {
                                return BackendResponse::err(&"Previous computation is still running. Cancel it before starting a new one.".to_string());
                            }
                        }
                        let mut new_cmp = Computation {
                            timestamp: SystemTime::now(),
                            is_cancelled: Arc::new(AtomicBool::new(false)),
                            input_model: aeon_string.clone(),
                            graph: None,
                            classifier: None,
                            progress: None,
                            thread: None,
                            error: None,
                        };
                        let cancelled = new_cmp.is_cancelled.clone();
                        // Prepare thread - not that we have computation locked, so the thread
                        // will have to wait for us to end before writing down the graph and other
                        // stuff.
                        let cmp_thread = std::thread::spawn(move || {
                            let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
                            match AsyncGraph::new(network) {
                                Ok(graph) => {
                                    // Now that we have graph, we can create classifier and progress
                                    // and save them into the computation.
                                    let classifier = Arc::new(Classifier::new(&graph));
                                    let progress = Arc::new(ProgressTracker::new(&graph));
                                    let graph = Arc::new(graph);
                                    {
                                        if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                            cmp.graph = Some(graph.clone());
                                            cmp.progress = Some(progress.clone());
                                            cmp.classifier = Some(classifier.clone());
                                        } else {
                                            panic!("Cannot save graph. No computation found.")
                                        }
                                    }

                                    // Now we can actually start the computation...
                                    components(&graph, &progress, &*cancelled, |component| {
                                        let size = component.iter().count();
                                        println!("Component {}", size);
                                        classifier.add_component(component, &graph);
                                    });

                                    println!("Component search done...");
                                }
                                Err(error) => {
                                    if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                        cmp.error = Some(error);
                                    } else {
                                        panic!(
                                            "Cannot save computation error. No computation found."
                                        )
                                    }
                                }
                            }
                            {
                                // Remove reference to thread, since we are done now...
                                if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                    cmp.thread = None;
                                } else {
                                    panic!("Cannot finalize thread. No computation found.");
                                }
                            }
                            return ();
                        });
                        new_cmp.thread = Some(cmp_thread);

                        let start = new_cmp.start_timestamp();
                        // Now write the new computation to the global state...
                        *cmp = Some(new_cmp);

                        BackendResponse::ok(&object!{ "timestamp" => start as u64 }.to_string()) // status of the computation can be obtained via ping...
                    }
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

#[post("/cancel_computation", format = "plain")]
fn cancel_computation() -> BackendResponse {
    let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
    {
        // first just check there is something to cancel
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if cmp.thread.is_none() {
                return BackendResponse::err(
                    &"Nothing to cancel. Computation already done.".to_string(),
                );
            }
            if cmp.is_cancelled.load(Ordering::SeqCst) {
                return BackendResponse::err(&"Computation already cancelled.".to_string());
            }
        } else {
            return BackendResponse::err(&"No computation to cancel.".to_string());
        }
    }
    let cmp = cmp.write().unwrap();
    if let Some(cmp) = &*cmp {
        if cmp.thread.is_none() {
            return BackendResponse::err(
                &"Nothing to cancel. Computation already done.".to_string(),
            );
        }
        if cmp.is_cancelled.swap(true, Ordering::SeqCst) == false {
            return BackendResponse::ok(&"\"ok\"".to_string());
        } else {
            return BackendResponse::err(&"Computation already cancelled.".to_string());
        }
    } else {
        return BackendResponse::err(&"No computation to cancel.".to_string());
    }
}

/// Accept an SBML (XML) file and try to parse it into a `BooleanNetwork`.
/// If everything goes well, return a standard result object with a parsed model, or
/// error if something fails.
#[post("/sbml_to_aeon", format = "plain", data = "<data>")]
fn sbml_to_aeon(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut sbml_string = String::new();
    return match stream.read_to_string(&mut sbml_string) {
        Ok(_) => {
            match BooleanNetwork::from_sbml(&sbml_string) {
                Ok((model, layout)) => {
                    let mut model_string = format!("{}", model); // convert back to aeon
                    model_string += "\n";
                    for (var, (x, y)) in layout {
                        model_string += format!("#position:{}:{},{}\n", var, x, y).as_str();
                    }
                    BackendResponse::ok(&object! { "model" => model_string }.to_string())
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

/// Try to read the model layout metadata from the given aeon file.
fn read_layout(aeon_string: &str) -> HashMap<String, (f64, f64)> {
    let re = Regex::new(
        r"^\s*#position:(?P<var>[a-zA-Z0-9_]+):(?P<x>[+-]?\d+(\.\d+)?),(?P<y>[+-]?\d+(\.\d+)?)\s*$",
    )
    .unwrap();
    let mut layout = HashMap::new();
    for line in aeon_string.lines() {
        if let Some(captures) = re.captures(line) {
            let var = captures["var"].to_string();
            let x = captures["x"].parse::<f64>().unwrap();
            let y = captures["y"].parse::<f64>().unwrap();
            layout.insert(var, (x, y));
        }
    }
    return layout;
}

fn read_metadata(aeon_string: &str) -> (Option<String>, Option<String>) {
    let mut model_name = None;
    let mut model_description = None;
    let name_regex = Regex::new(r"^\s*#name:(?P<name>.+)$").unwrap();
    let description_regex = Regex::new(r"^\s*#description:(?P<desc>.+)$").unwrap();
    for line in aeon_string.lines() {
        if let Some(captures) = name_regex.captures(line) {
            model_name = Some(captures["name"].to_string());
        }
        if let Some(captures) = description_regex.captures(line) {
            model_description = Some(captures["desc"].to_string());
        }
    }
    return (model_name, model_description);
}

/// Accept an Aeon file, try to parse it into a `BooleanNetwork`
/// which will then be translated into SBML (XML) representation.
/// Preserve layout metadata.
#[post("/aeon_to_sbml", format = "plain", data = "<data>")]
fn aeon_to_sbml(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => match BooleanNetwork::try_from(aeon_string.as_str()) {
            Ok(network) => {
                let layout = read_layout(&aeon_string);
                let sbml_string = network.to_sbml(&layout);
                BackendResponse::ok(&object! { "model" => sbml_string }.to_string())
            }
            Err(error) => BackendResponse::err(&error),
        },
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

/// Accept an Aeon file and create an SBML version with all parameters instantiated (a witness model).
/// Note that this can take quite a while for large models since we have to actually build
/// the unit BDD right now (in the future, we might opt to use a SAT solver which might be faster).
#[post("/aeon_to_sbml_instantiated", format = "plain", data = "<data>")]
fn aeon_to_sbml_instantiated(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => {
            match BooleanNetwork::try_from(aeon_string.as_str()).and_then(|bn| AsyncGraph::new(bn))
            {
                Ok(graph) => {
                    let witness = graph.make_witness(graph.unit_params());
                    let layout = read_layout(&aeon_string);
                    BackendResponse::ok(
                        &object! { "model" => witness.to_sbml(&layout) }.to_string(),
                    )
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

fn main() {
    //test_main::run();
    rocket::ignite()
        .mount(
            "/",
            routes![
                ping,
                start_computation,
                cancel_computation,
                get_results,
                get_witness,
                check_update_function,
                sbml_to_aeon,
                aeon_to_sbml,
                aeon_to_sbml_instantiated
            ],
        )
        .launch();
}

struct BackendResponse {
    message: String,
}

impl BackendResponse {
    fn ok(message: &String) -> Self {
        return BackendResponse {
            message: format!("{{ \"status\": true, \"result\": {} }}", message),
        };
    }

    fn err(message: &String) -> Self {
        return BackendResponse {
            message: format!("{{ \"status\": false, \"message\": \"{}\" }}", message),
        };
    }
}

impl<'r> Responder<'r> for BackendResponse {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        use std::io::Cursor;

        let cursor = Cursor::new(self.message);
        Response::build()
            .header(ContentType::Plain)
            .header(AccessControlAllowOrigin::Any)
            // This magic set of headers might fix some CROS issues, but we are not sure yet...
            .header(Header::new("Allow", "GET, POST, OPTIONS, PUT, DELETE"))
            .header(Header::new("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, DELETE"))
            .header(Header::new("Access-Control-Allow-Headers", "X-API-KEY, Origin, X-Requested-With, Content-Type, Accept, Access-Control-Request-Method"))
            .sized_body(cursor)
            .ok()
    }
}
