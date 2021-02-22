use crate::BackendResponse;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryFrom;

/// Accept an Aeon file, try to parse it into a `BooleanNetwork`
/// which will then be translated into SBML (XML) representation.
/// Preserve layout metadata.
pub fn aeon_to_sbml(aeon_string: &str) -> BackendResponse {
    match BooleanNetwork::try_from(aeon_string) {
        Ok(network) => {
            let layout = read_layout(&aeon_string);
            let sbml_string = network.to_sbml(Some(&layout));
            BackendResponse::ok_json(object! { "model" => sbml_string })
        }
        Err(error) => BackendResponse::err(&error),
    }
}

/// Try to read the model layout metadata from the given aeon file.
pub fn read_layout(aeon_string: &str) -> HashMap<String, (f64, f64)> {
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

/// Accept an Aeon file and create an SBML version with all parameters instantiated (a witness model).
/// Note that this can take quite a while for large models since we have to actually build
/// the unit BDD right now (in the future, we might opt to use a SAT solver which might be faster).
pub fn aeon_to_sbml_instantiated(aeon_string: &str) -> BackendResponse {
    match BooleanNetwork::try_from(aeon_string).and_then(|bn| AsyncGraph::new(bn)) {
        Ok(graph) => {
            let witness = graph.make_witness(graph.unit_params());
            let layout = read_layout(&aeon_string);
            let sbml_string = witness.to_sbml(Some(&layout));
            BackendResponse::ok_json(object! { "model" => sbml_string })
        }
        Err(error) => BackendResponse::err(&error),
    }
}
