#[macro_use]
extern crate json;

extern crate tinyfiledialogs as tfd;

use biodivine_aeon_server::scc::{Behaviour, Class, Classifier};
use biodivine_aeon_server::{BackendResponse, Computation, COMPUTATION};
use regex::Regex;

mod test_main;

use biodivine_aeon_server::requests::aeon_to_sbml::read_layout;
use biodivine_aeon_server::requests::process_request;
use biodivine_lib_param_bn::bdd_params::BddParams;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tfd::MessageBoxIcon;
use web_view::Content;

fn try_get_class_params(classifier: &Classifier, class: &Class) -> Option<Option<BddParams>> {
    for _ in 0..5 {
        if let Some(data) = classifier.try_get_params(class) {
            return Some(data);
        }
        // wait a little - maybe the lock will become free
        std::thread::sleep(Duration::new(1, 0));
    }
    return None;
}

//#[get("/get_witness/<class_str>")]
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
                if let Some(has_class) = try_get_class_params(classifier, &class) {
                    if let Some(class) = has_class {
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
                        return BackendResponse::err(
                            &"Specified class has no witness.".to_string(),
                        );
                    }
                } else {
                    return BackendResponse::err(
                        &"Classification in progress. Cannot extract witness right now."
                            .to_string(),
                    );
                }
            } else {
                return BackendResponse::err(&"No results available.".to_string());
            }
        } else {
            return BackendResponse::err(&"No results available.".to_string());
        }
    }
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

fn main() {
    let index_str = include_str!("../index.dev.bundle.html");

    let webview = web_view::builder()
        .title("Aeon 2019")
        .content(Content::Html(index_str))
        .size(1200, 800)
        .resizable(true)
        //.debug(true)
        .user_data(())
        .invoke_handler(|webview, arg| {
            println!("Request: {}", arg);
            match json::parse(arg) {
                Ok(json) => {
                    let id: u64 = json["requestId"].as_u64().unwrap();
                    let path: &str = json["path"].as_str().unwrap();
                    if path == "log" {
                        println!("{}", json["message"]);
                        Ok(())
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
                        let mut response = process_request(path, &json).json();
                        response["requestId"] = id.into();
                        let command =
                            format!("NativeBridge.handleResponse({})", json::stringify(response));
                        println!("Command: {}", command);
                        webview.eval(&command)
                    }
                }
                Err(e) => panic!("Invalid native request: {:?}", e),
            }
        })
        .build()
        .unwrap();

    webview.run().unwrap();
}
