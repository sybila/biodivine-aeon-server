use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::{BooleanNetwork, RegulatoryGraph};
use std::convert::TryFrom;
use std::io::Read;
use biodivine_aeon_server::algorithms::attractors::transition_guided_reduction;

#[tokio::main]
async fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    let model = inline_inputs(model);
    println!("Model loaded. {} variables and {} parameters.", model.num_vars(), model.num_parameters());

    let graph = SymbolicAsyncGraph::new(model).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrisations: {}",
        graph.unit_colors().approx_cardinality()
    );
    println!(
        "State space: {}",
        graph.unit_colored_vertices().approx_cardinality()
    );

    // First, perform ITGR reduction.
    let universe = transition_guided_reduction(&graph, num_cpus::get_physical()).await;

    println!("Universe: {}", universe.approx_cardinality());
}

fn inline_inputs(bn: BooleanNetwork) -> BooleanNetwork {
    let mut variables = Vec::new();
    let mut parameters = Vec::new();
    for var in bn.variables() {
        if bn.regulators(var).len() == 0 {
            parameters.push(bn.get_variable_name(var).clone());
        } else {
            variables.push(bn.get_variable_name(var).clone());
        }
    }

    let mut inlined_rg = RegulatoryGraph::new(variables.clone());

    for reg in bn.as_graph().regulations() {
        let old_regulator = bn.get_variable_name(reg.get_regulator());
        let old_target = bn.get_variable_name(reg.get_target());
        if variables.contains(old_regulator) {
            inlined_rg.add_regulation(
                old_regulator,
                old_target,
                false,
                reg.get_monotonicity()
            ).unwrap();
        }
    }

    let mut inlined_bn = BooleanNetwork::new(inlined_rg);

    for param in parameters {
        inlined_bn.add_parameter(param.as_str(), 0).unwrap();
    }

    for var in inlined_bn.variables() {
        let name = inlined_bn.get_variable_name(var).clone();
        let old_id = bn.as_graph().find_variable(name.as_str()).unwrap();
        let old_function = bn.get_update_function(old_id).as_ref().unwrap();
        inlined_bn.add_string_update_function(name.as_str(), old_function.to_string(&bn).as_str()).unwrap();
    }

    inlined_bn
}
