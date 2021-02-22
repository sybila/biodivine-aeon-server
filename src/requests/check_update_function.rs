use crate::BackendResponse;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use std::convert::TryFrom;

/// Accept a partial model containing only the necessary regulations and one update function.
/// Return cardinality of such model (i.e. the number of instantiations of this update function)
/// or error if the update function (or model) is invalid.
pub fn check_update_function(model_string: &str) -> BackendResponse {
    let graph = BooleanNetwork::try_from(model_string).and_then(|model| {
        if model.as_graph().num_vars() <= 5 {
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
