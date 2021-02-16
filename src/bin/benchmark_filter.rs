use biodivine_lib_param_bn::{BooleanNetwork, RegulatoryGraph};

fn main() {
    let benchmarks = std::fs::read_dir("./benchmark").unwrap();
    let current_dir = std::env::current_dir().unwrap();
    let aeon_benchmarks = current_dir.join("aeon_models");
    if !aeon_benchmarks.exists() {
        std::fs::create_dir_all(&aeon_benchmarks).unwrap();
    }

    let mut i = 0;
    for bench_dir in benchmarks {
        let bench_dir = bench_dir.unwrap();
        if !bench_dir.file_type().unwrap().is_dir() {
            continue;
        }
        i += 1;

        let model_path = bench_dir.path().join("model.sbml");
        let model_string = std::fs::read_to_string(model_path).unwrap();
        let (sbml_model, _) = BooleanNetwork::from_sbml(&model_string).unwrap();
        let model = erase_regulation_bounds(&sbml_model);

        let bench_name = bench_dir.file_name().to_str().unwrap().to_string();
        let aeon_file = aeon_benchmarks.join(&format!("{}_{}.aeon", i, bench_name));
        std::fs::write(aeon_file, model.to_string()).unwrap();
    }
}

/// This will erase the observability/monotonicity requirements for regulations, because we don't
/// need them in systems without parameters.
///
/// Also erases any pre-set input constants.
fn erase_regulation_bounds(network: &BooleanNetwork) -> BooleanNetwork {
    let variable_names = network
        .variables()
        .map(|v| network.get_variable_name(v).to_string())
        .collect();
    let mut rg = RegulatoryGraph::new(variable_names);
    for old_reg in network.as_graph().regulations() {
        rg.add_regulation(
            network.get_variable_name(old_reg.get_regulator()),
            network.get_variable_name(old_reg.get_target()),
            false,
            None,
        )
        .unwrap();
    }
    let mut bn = BooleanNetwork::new(rg);
    for old_v in network.variables() {
        let new_v = bn
            .as_graph()
            .find_variable(network.get_variable_name(old_v))
            .unwrap();
        let is_input = network.regulators(old_v).is_empty();
        if !is_input {
            if let Some(fn_update) = network.get_update_function(old_v).clone() {
                bn.add_update_function(new_v, fn_update).unwrap();
            }
        }
    }
    bn
}
