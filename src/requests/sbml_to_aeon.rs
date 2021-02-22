use crate::BackendResponse;
use biodivine_lib_param_bn::BooleanNetwork;

/// Accept an SBML (XML) file and try to parse it into a `BooleanNetwork`.
/// If everything goes well, return a standard result object with a parsed model, or
/// error if something fails.
pub fn sbml_to_aeon(sbml_string: &str) -> BackendResponse {
    match BooleanNetwork::try_from_sbml(&sbml_string) {
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
