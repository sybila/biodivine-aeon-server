use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::{BooleanNetwork, RegulatoryGraph};
use std::convert::TryFrom;
use std::io::Read;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_aeon_server::algorithms::attractors::transition_guided_reduction;
use biodivine_aeon_server::algorithms::reachability::bwd;

#[tokio::main]
async fn main() {
    //let mut buffer = String::new();
    //std::io::stdin().read_to_string(&mut buffer).unwrap();

    let args = std::env::args().collect::<Vec<_>>();
    let buffer = std::fs::read_to_string(args[1].as_str()).unwrap();

    let model = BooleanNetwork::try_from_bnet(buffer.as_str()).unwrap();
    let model = inline_inputs(model);
    println!("Model loaded. {} variables and {} parameters.", model.num_vars(), model.num_parameters());

    let graph = SymbolicAsyncGraph::new(model.clone()).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrisations: {}",
        graph.unit_colors().approx_cardinality()
    );
    println!(
        "State space: {}",
        graph.unit_colored_vertices().approx_cardinality()
    );

    let mut sinks = graph.mk_unit_colored_vertices();

    /*for var in model.variables() {
        let can_post = graph.var_can_post(var, &sinks);
        sinks = sinks.minus(&can_post);
        println!("Applied {:?}, result is {} / {}", var, sinks.approx_cardinality(), sinks.symbolic_size());
    }*/

    let mut candidates = model.variables().collect::<Vec<_>>();
    while !candidates.is_empty() {
        let mut best = (usize::MAX, model.variables().next().unwrap());
        for var in &candidates {
            let can_post = graph.var_can_post(*var, &sinks);
            let result = sinks.minus(&can_post);
            if result.symbolic_size() < best.0 {
                best = (result.symbolic_size(), *var);
            }
        }
        let best = best.1;
        let index = candidates.iter().position(|x| *x == best).unwrap();
        candidates.remove(index);
        let can_post = graph.var_can_post(best, &sinks);
        sinks = sinks.minus(&can_post);
        println!("Applied {:?} ({}), result is {} / {}", best, candidates.len(), sinks.approx_cardinality(), sinks.symbolic_size());
    }

    println!("Sinks: {}", sinks.approx_cardinality());

    let variables = graph.as_network().variables().collect::<Vec<_>>();
    let basin = bwd(&graph, &sinks, &variables).await;
    println!("Basin: {} / {}", basin.approx_cardinality(), basin.symbolic_size());
    let not_basin = graph.unit_colored_vertices().minus(&basin);
    println!("Not basin: {} / {}", not_basin.approx_cardinality(), not_basin.symbolic_size());
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
