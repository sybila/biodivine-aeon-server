#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::http::hyper::header::AccessControlAllowOrigin;
use rocket::http::ContentType;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use crate::scc::{Class, Classifier};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use scc::algo_components::components;
use std::convert::TryFrom;

mod scc;
mod test_main;

use biodivine_lib_param_bn::bdd_params::BddParams;
use std::collections::HashMap;

static mut SERVER_STATE: ServerState = ServerState::new();

struct ServerState {
    is_busy: bool,
    model: Option<BooleanNetwork>,
    result: Option<HashMap<Class, BddParams>>,
}

impl ServerState {
    pub const fn new() -> Self {
        ServerState {
            is_busy: false,
            model: None,
            result: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy
    }

    pub fn set_model(&mut self, model: &BooleanNetwork) {
        self.model = Some(model.clone());
        self.result = None;
    }

    pub fn get_result(&mut self) -> Option<&HashMap<Class, BddParams>> {
        if let None = self.result {
            self.compute()
        }

        self.result.as_ref()
    }

    pub fn get_model(&self) -> Option<&BooleanNetwork> {
        self.model.as_ref()
    }

    fn set_busy(&mut self, new: bool) {
        self.is_busy = new;
    }

    fn compute(&mut self) {
        if let None = self.model {
            return;
        }

        self.set_busy(true);
        let model = self.model.as_ref().unwrap().clone();
        let graph = AsyncGraph::new(model).unwrap(); // TODO

        let mut classifier = Classifier::new(&graph);
        components(&graph, |component| {
            let size = component.iter().count();
            println!("Component {}", size);
            classifier.add_component(component);
        });

        self.result = Some(classifier.export_result());
        self.set_busy(false);
    }
}

struct BackendResponse {
    message: String,
}

impl BackendResponse {
    fn new(message: &String) -> Self {
        BackendResponse {
            message: message.clone(),
        }
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
    unsafe {
        let (ru, mo, re) = (
            SERVER_STATE.is_busy,
            SERVER_STATE.model.is_some(),
            SERVER_STATE.result.is_some(),
        );

        BackendResponse::new(&format!(
            "{{\"busy\": {}, \"has_model\": {}, \"has_result\": {}}}",
            ru, mo, re
        ))
    }
}

#[get("/get_result")]
fn get_result() -> BackendResponse {
    let result = unsafe { SERVER_STATE.get_result() };
    if let None = result {
        return BackendResponse::new(&String::from("None"));
    }

    let lines: Vec<String> = result
        .unwrap()
        .iter()
        .map(|(c, p)| format!("{{\"sat_count\":{},\"phenotype\":{}}}", p.cardinality(), c))
        .collect();

    println!("Result {:?}", lines);

    let mut json = String::new();
    for index in 0..lines.len() - 1 {
        json += &format!("{},", lines[index]);
    }
    json = format!("{{\"result\":[{}{}]}}", json, lines.last().unwrap());

    BackendResponse::new(&json)
}

#[get("/get_model")]
fn get_model() -> BackendResponse {
    unsafe {
        BackendResponse::new(&match SERVER_STATE.get_model() {
            None => String::from("None"),
            Some(model) => model.to_string(),
        })
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
    println!("Boolnet: {}", boolnet);
    unsafe {
        if SERVER_STATE.is_busy() {
            BackendResponse::new(&"server is busy.".to_string())
        } else {
            let bn = BooleanNetwork::try_from(boolnet.as_str());
            match bn {
                Ok(bn) => {
                    SERVER_STATE.set_model(&bn);
                    println!("Set model result ok");
                    BackendResponse::new(&"Ok".to_string())
                }
                Err(message) => {
                    println!("Set model result err: {}", message);
                    BackendResponse::new(&message)
                }
            }
        }
    }
}

fn main() {
    test_main::run();
    /*rocket::ignite()
    .mount(
        "/",
        routes![get_info, get_model, get_result, set_model, sbml_to_aeon],
    )
    .launch();*/
}
