#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

use rocket::http::hyper::header::AccessControlAllowOrigin;
use rocket::http::ContentType;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use crate::scc::{Class, Classifier, Behaviour};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use scc::algo_components::components;
use std::convert::TryFrom;

pub mod scc;
mod test_main;

use biodivine_lib_param_bn::bdd_params::BddParams;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use rocket::Data;
use std::io::Read;

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

/*#[post("/sbml_to_aeon", data = "<data>")]
fn sbml_to_aeon(data: String) -> BackendResponse {
    println!("data was {}", data);

    /*let boolnet = BooleanNetwork::from_sbml(&data);
    match boolnet {
        //Ok(result) => BackendResponse::new(&String::from(result.to_string())),
        _ => BackendResponse::new(&String::from("None")),
    }*/
    BackendResponse::new(&String::from("None"))
}*/

#[get("/set_model/<boolnet>")]
fn set_model(boolnet: String) -> BackendResponse {
    let set_model_result = { SERVER_STATE.lock().unwrap().set_model(boolnet.as_str()) };
    return match set_model_result {
        Ok(()) => BackendResponse::ok(&"\"\"".to_string()),
        Err(message) => BackendResponse::err(&message),
    };
}

/// Accept a partial model containing only the necessary regulations and one update function.
/// Return cardinality of such model (i.e. the number of instantiations of this update function)
/// or error if the update function (or model) is invalid.
#[post("/check_update_function", format="plain", data="<data>")]
fn check_update_function(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000);    // limit model size to 10MB
    let mut model_string = String::new();
    return match stream.read_to_string(&mut model_string) {
        Ok(_) => {
            println!("Check update function: ");
            println!("{}", model_string);
            match BooleanNetwork::try_from(model_string.as_str()).and_then(|model| AsyncGraph::new(model)) {
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
    return BackendResponse::ok(&"\"Ok\"".to_string());
}

fn main() {
    //test_main::run();
    rocket::ignite()
        .mount("/", routes![ping,check_update_function])
        .launch();
}
