use crate::{ArcComputation, BackendResponse, OPEN_MODEL};
use aeon_to_sbml::{aeon_to_sbml, aeon_to_sbml_instantiated};
use check_update_function::check_update_function;
use get_results::get_results;
use get_witness::get_witness;
use json::JsonValue;
use ping::ping;
use sbml_to_aeon::sbml_to_aeon;

pub mod aeon_to_sbml;
pub mod check_update_function;
pub mod computation;
pub mod get_results;
pub mod get_witness;
pub mod ping;
pub mod sbml_to_aeon;

pub fn process_request(session: ArcComputation, path: &str, data: &JsonValue) -> BackendResponse {
    match path {
        "ping" => ping(session),
        "get_results" => get_results(session),
        "cancel_computation" => computation::cancel_computation(session),
        "start_computation" => {
            computation::start_computation(session, data["aeonString"].as_str().unwrap())
        }
        "check_update_function" => check_update_function(data["modelFragment"].as_str().unwrap()),
        "sbml_to_aeon" => sbml_to_aeon(data["sbmlString"].as_str().unwrap()),
        "aeon_to_sbml" => aeon_to_sbml(data["aeonString"].as_str().unwrap()),
        "aeon_to_sbml_instantiated" => {
            aeon_to_sbml_instantiated(data["aeonString"].as_str().unwrap())
        }
        "open_witness" => {
            println!("Try lock to get sender.");
            let send = { (*OPEN_MODEL.lock().unwrap()).clone() };
            if let Some(send) = send {
                if let Some(model) = get_witness(session, data["witness"].as_str().unwrap()) {
                    send.send(model).unwrap();
                }
            }
            BackendResponse::ok_json(JsonValue::from("ok"))
        }
        _ => BackendResponse::err(&"Unimplemented".to_string()),
    }
}
