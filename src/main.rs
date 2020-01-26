#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate json;

use rocket::http::hyper::header::AccessControlAllowOrigin;
use rocket::http::ContentType;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use crate::scc::{Classifier, ProgressTracker};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use std::convert::TryFrom;
use regex::Regex;

pub mod scc;
mod test_main;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use rocket::Data;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use crate::scc::algo_components::components;
/*
lazy_static! {
    static ref SERVER_STATE: Mutex<ServerState> = Mutex::new(ServerState::new());
}

//static mut SERVER_STATE: ServerState = ServerState::new();

struct ServerState {
    is_busy: bool,  // true = computation is in progress
    model: Option<BooleanNetwork>,  // Some(..) = model is set
    result: Option<Arc<(AsyncGraph, Classifier)>>,     // Some(..) = computation is in progress or done (if is_busy = false)
}

impl ServerState {
    pub const fn new() -> Self {
        ServerState {
            is_busy: false,
            model: None,
            result: None,
        }
    }

    /// Check if computation is running in some thread.
    pub fn is_busy(&self) -> bool {
        self.is_busy
    }

    /// Update model
    pub fn set_model(&mut self, model: &str) -> Result<(), String> {
        if self.is_busy {
            // If we are computing, we can't delete classifier because somebody is using it
            return Err("Computation in progress".to_string());
        } else {
            let bn = BooleanNetwork::try_from(model)?;
            self.model = Some(bn);
            self.result = None;
            return Ok(());
        }
    }

    pub fn get_result(&self) -> Option<HashMap<Class, BddParams>> {
        return self.result.as_ref().map(|c| c.1.export_result());
    }

    pub fn get_model(&self) -> Option<BooleanNetwork> {
        return self.model.as_ref().map(|m| m.clone());
    }

    fn set_busy(&mut self, new: bool) {
        self.is_busy = new;
    }

    /// Start computation in a background thread, assuming it is not already running.
    pub fn start_compute(&mut self) -> Result<(), String> {
        if self.is_busy {
            return Err("Computation in progress.".to_string());
        }
        return if let Some(model) = &self.model {
            self.is_busy = true;
            match AsyncGraph::new(model.clone()) {
                Ok(graph) => {
                    let classifier = Classifier::new(&graph);
                    self.result = Some(Arc::new((graph, classifier)));

                    let ctx = self.result.as_ref().unwrap().clone();
                    //std::thread::spawn(move || {
                        components(&ctx.0, |component| {
                            let size = component.iter().count();
                            println!("Component {}", size);
                            ctx.1.add_component(component, &ctx.0);
                        });
                    self.is_busy = false;
                        //SERVER_STATE.lock().unwrap().is_busy = false;   // mark as done
                    //});
                }
                Err(message) => {
                    self.is_busy = false;
                    return Err(message)
                }
            }
            Ok(())
        } else {
            Err("No model set.".to_string())
        }
    }

    pub fn make_witness(&mut self, class: &Class) -> Result<BooleanNetwork, String> {
        if self.is_busy {
            return Err("Computation in progress.".to_string());
        }
        let result = self.result.as_ref();
        if let Some(result) = result {
            let result_data = self.get_result();
            if let Some(result_data) = result_data {
                if let Some(result_params) = result_data.get(class) {
                    let witness = result.clone().0.make_witness(result_params);
                    return Ok(witness);
                } else {
                    return Err(format!("No witnesses for class {}", class))
                }
            } else {
                return Err("Results not available. Cannot make witness.".to_string())
            }
        } else {
            return Err("Results not available. Cannot make witness.".to_string())
        }
    }

}

#[get("/get_info")]
fn get_info() -> BackendResponse {
    let state = SERVER_STATE.lock().unwrap();
    let (ru, mo, re) = (
        state.is_busy,
        state.model.is_some(),
        state.result.is_some(),
    );

    BackendResponse::ok(&format!(
        "{{\"busy\": {}, \"has_model\": {}, \"has_result\": {}}}",
        ru, mo, re
    ))
}

#[get("/get_result")]
fn get_result() -> BackendResponse {
    let result = { SERVER_STATE.lock().unwrap().get_result() };
    if result.is_none() {
        // start compute is now blocking...
        SERVER_STATE.lock().unwrap().start_compute();
    }
    let result = { SERVER_STATE.lock().unwrap().get_result() };
    return if let Some(data) = result {
        let lines: Vec<String> = data
            .iter()
            .map(|(c, p)| format!("{{\"sat_count\":{},\"phenotype\":{}}}", p.cardinality(), c))
            .collect();

        println!("Result {:?}", lines);

        let mut json = String::new();
        for index in 0..lines.len() - 1 {
            json += &format!("{},", lines[index]);
        }
        json = format!("[{}{}]", json, lines.last().unwrap());

        BackendResponse::ok(&json)
    } else {
        // (if the computation is running, we won't be able to start a new one)
        let computation_started = { SERVER_STATE.lock().unwrap().start_compute() };
        match computation_started {
            Ok(()) => BackendResponse::err(&"Result not available. Computation started.".to_string()),
            Err(message) => BackendResponse::err(&message),
        }
    }
}

#[get("/get_model/<class_str>")]
fn get_model(class_str: String) -> BackendResponse {
    return if class_str == "original".to_string() {
        let model = { SERVER_STATE.lock().unwrap().get_model() };
        if let Some(model) = model {
            BackendResponse::ok(&format!("\"{}\"", model).replace('\n', "\\n"))
        } else {
            BackendResponse::err(&"No model set.".to_string())
        }
    } else {
        let mut class = Class::new_empty();
        for char in class_str.chars() {
            match char {
                'D' => class.extend(Behaviour::Disorder),
                'O' => class.extend(Behaviour::Oscillation),
                'S' => class.extend(Behaviour::Stability),
                _ => {
                    return BackendResponse::err(&"Invalid class.".to_string())
                }
            }
        }
        let witness = { SERVER_STATE.lock().unwrap().make_witness(&class) };
        match witness {
            Ok(network) => BackendResponse::ok(&format!("\"{}\"", network).replace('\n', "\\n")),
            Err(message) => BackendResponse::err(&message),
        }
    }
}

*/

/// Computation keeps all information
struct Computation {
    is_cancelled: AtomicBool,   // indicate to the server that the computation should be cancelled
    input_model: String,        // .aeon string representation of the model
    graph: Option<Arc<AsyncGraph>>,          // Model graph - used to create witnesses
    classifier: Option<Arc<Classifier>>,     // Classifier used to store the results of the computation
    progress: Option<Arc<ProgressTracker>>,  // Used to access progress of the computation
    thread: Option<JoinHandle<()>>,     // A thread that is actually doing the computation (so that we can check if it is still running). If none, the computation is done.
    error: Option<String>,              // A string error from the computation
}

lazy_static! {
    static ref COMPUTATION: Arc<RwLock<Option<Computation>>> = Arc::new(RwLock::new(None));
}

/// Accept a partial model containing only the necessary regulations and one update function.
/// Return cardinality of such model (i.e. the number of instantiations of this update function)
/// or error if the update function (or model) is invalid.
/// TODO: On some large models, this sometimes returns some bogus number even though the model is too large to run :O
#[post("/check_update_function", format="plain", data="<data>")]
fn check_update_function(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000);    // limit model size to 10MB
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
                Ok(graph) => {
                    BackendResponse::ok(&format!("{{\"cardinality\":\"{}\"}}", graph.unit_params().cardinality()))
                }
                Err(error) => BackendResponse::err(&error)
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error))
    }
}

#[get("/ping")]
fn ping() -> BackendResponse {
    println!("...ping...");
    let mut response = object!{
        "has_computation" => false,
        "is_cancelled" => false,
        "running" => false,
        "progress" => "unknown".to_string(),
        "error" => json::Null,
    };
    {   // Read data from current computation if available...
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        // TODO: Computation contains classifier that can be locked for quite some time
        // Maybe that one should have a separate lock so that it does not block ping.
        let cmp = cmp.read().unwrap();
        if let Some(computation) = &*cmp {
            response["has_computation"] = true.into();
            response["is_cancelled"] = computation.is_cancelled.load(Ordering::SeqCst).into();
            if let Some(progress) = &computation.progress {
                response["progress"] = progress.get_percent_string().into();
            }
            response["is_running"] = computation.thread.is_some().into();
            if let Some(error) = &computation.error {
                response["error"] = error.clone().into();
            }
        }
    }
    return BackendResponse::ok(&response.to_string());
}

/// Accept an Aeon model, parse it and start a new computation (if there is no computation running).
#[post("/start_computation", format="plain", data="<data>")]
fn start_computation(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000);  // limit model to 10MB
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
                            is_cancelled: AtomicBool::new(false),
                            input_model: aeon_string.clone(),
                            graph: None, classifier: None,
                            progress: None, thread: None,
                            error: None,
                        };
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
                                    components(&graph, &progress, |component| {
                                        classifier.add_component(component, &graph);
                                    });

                                    println!("Component search done...");
                                }
                                Err(error) => {
                                    {
                                        if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                            cmp.error = Some(error);
                                        } else {
                                            panic!("Cannot save computation error. No computation found.")
                                        }
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

                        // Now write the new computation to the global state...
                        *cmp = Some(new_cmp);

                        BackendResponse::ok(&"\"Ok\"".to_string())  // status of the computation can be obtained via ping...
                    }
                }
                Err(error) => BackendResponse::err(&error)
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error))
    };
}

/// Accept an SBML (XML) file and try to parse it into a `BooleanNetwork`.
/// If everything goes well, return a standard result object with a parsed model, or
/// error if something fails.
#[post("/sbml_to_aeon", format="plain", data="<data>")]
fn sbml_to_aeon(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000);  // limit model to 10MB
    let mut sbml_string = String::new();
    return match stream.read_to_string(&mut sbml_string) {
        Ok(_) => {
            match BooleanNetwork::from_sbml(&sbml_string) {
                Ok((model, layout)) => {
                    let mut model_string = format!("{}", model);    // convert back to aeon
                    model_string += "\n";
                    for (var, (x,y)) in layout {
                        model_string += format!("#position:{}:{},{}\n", var, x, y).as_str();
                    }
                    BackendResponse::ok(&object!{ "model" => model_string }.to_string())
                }
                Err(error) => BackendResponse::err(&error)
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error))
    }
}

/// Try to read the model layout metadata from the given aeon file.
fn read_layout(aeon_string: &str) -> HashMap<String, (f64, f64)> {
    let re = Regex::new(r"^\s*#position:(?P<var>[a-zA-Z0-9_]+):(?P<x>[+-]?\d+(\.\d+)?),(?P<y>[+-]?\d+(\.\d+)?)\s*$").unwrap();
    let mut layout = HashMap::new();
    for line in aeon_string.lines() {
        if let Some(captures) = re.captures(line) {
            let var = captures["var"].to_string();
            let x = captures["x"].parse::<f64>().unwrap();
            let y = captures["y"].parse::<f64>().unwrap();
            layout.insert(var, (x,y));
        }
    }
    return layout;
}

/// Accept an Aeon file, try to parse it into a `BooleanNetwork`
/// which will then be translated into SBML (XML) representation.
/// Preserve layout metadata.
#[post("/aeon_to_sbml", format="plain", data="<data>")]
fn aeon_to_sbml(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000);  // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => {
            match BooleanNetwork::try_from(aeon_string.as_str()) {
                Ok(network) => {
                    let layout = read_layout(&aeon_string);
                    let sbml_string = network.to_sbml(&layout);
                    BackendResponse::ok(&object!{ "model" => sbml_string }.to_string())
                }
                Err(error) => BackendResponse::err(&error)
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error))
    }
}

/// Accept an Aeon file and create an SBML version with all parameters instantiated (a witness model).
/// Note that this can take quite a while for large models since we have to actually build
/// the unit BDD right now (in the future, we might opt to use a SAT solver which might be faster).
#[post("/aeon_to_sbml_instantiated", format="plain", data="<data>")]
fn aeon_to_sbml_instantiated(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000);  // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => {
            match BooleanNetwork::try_from(aeon_string.as_str()).and_then(|bn| AsyncGraph::new(bn)) {
                Ok(graph) => {
                    let witness = graph.make_witness(graph.unit_params());
                    let layout = read_layout(&aeon_string);
                    BackendResponse::ok(&object! { "model" => witness.to_sbml(&layout) }.to_string())
                }
                Err(error) => BackendResponse::err(&error)
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error))
    };
}

fn main() {
    //test_main::run();
    rocket::ignite()
        .mount("/", routes![
            ping,
            start_computation,
            check_update_function,
            sbml_to_aeon,
            aeon_to_sbml,
            aeon_to_sbml_instantiated
        ])
        .launch();
}


struct BackendResponse {
    message: String,
}

impl BackendResponse {
    fn ok(message: &String) -> Self {
        return BackendResponse { message: format!("{{ \"status\": true, \"result\": {} }}", message) };
    }

    fn err(message: &String) -> Self {
        return BackendResponse { message: format!("{{ \"status\": false, \"message\": \"{}\" }}", message) };
    }
}

impl<'r> Responder<'r> for BackendResponse {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        use std::io::Cursor;

        let cursor = Cursor::new(self.message);
        Response::build()
            .header(ContentType::Plain)
            .header(AccessControlAllowOrigin::Any)
            .sized_body(cursor)
            .ok()
    }
}