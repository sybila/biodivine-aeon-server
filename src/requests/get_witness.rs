use crate::requests::aeon_to_sbml::read_layout;
use crate::scc::{Behaviour, Class, Classifier};
use crate::ArcComputation;
use biodivine_lib_param_bn::bdd_params::BddParams;
use regex::Regex;
use std::time::Duration;

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
pub fn get_witness(cmp: ArcComputation, class_str: &str) -> Option<String> {
    let mut class = Class::new_empty();
    for char in class_str.chars() {
        match char {
            'D' => class.extend(Behaviour::Disorder),
            'O' => class.extend(Behaviour::Oscillation),
            'S' => class.extend(Behaviour::Stability),
            _ => {
                return None;
            }
        }
    }
    {
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
                            Some(model_string)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
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
