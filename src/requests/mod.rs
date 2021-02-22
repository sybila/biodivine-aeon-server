use crate::BackendResponse;
use aeon_to_sbml::{aeon_to_sbml, aeon_to_sbml_instantiated};
use check_update_function::check_update_function;
use get_results::get_results;
use json::JsonValue;
use ping::ping;
use sbml_to_aeon::sbml_to_aeon;

pub mod aeon_to_sbml;
pub mod check_update_function;
pub mod computation;
pub mod get_results;
pub mod ping;
pub mod sbml_to_aeon;

pub fn process_request(path: &str, data: &JsonValue) -> BackendResponse {
    match path {
        "ping" => ping(),
        "get_results" => get_results(),
        "cancel_computation" => computation::cancel_computation(),
        "start_computation" => computation::start_computation(data["aeonString"].as_str().unwrap()),
        "check_update_function" => check_update_function(data["modelFragment"].as_str().unwrap()),
        "sbml_to_aeon" => sbml_to_aeon(data["sbmlString"].as_str().unwrap()),
        "aeon_to_sbml" => aeon_to_sbml(data["aeonString"].as_str().unwrap()),
        "aeon_to_sbml_instantiated" => {
            aeon_to_sbml_instantiated(data["aeonString"].as_str().unwrap())
        }
        _ => BackendResponse::err(&"Unimplemented".to_string()),
    }
}
