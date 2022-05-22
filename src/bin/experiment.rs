use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;
use biodivine_aeon_server::scc::algo_xie_beerel::xie_beerel_attractors;
use biodivine_aeon_server::scc::Classifier;
use biodivine_aeon_server::GraphTaskContext;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::{BooleanNetwork, RegulatoryGraph};
use std::convert::TryFrom;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_millis();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    let model = inline_inputs(model);
    let names: Vec<_> = model
        .variables()
        .map(|id| model.get_variable_name(id))
        .collect();
    println!("Model loaded...");
    println!("{} variables: {:?}", model.num_vars(), names);

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

    let classifier = Classifier::new(&graph);
    let task_context = GraphTaskContext::new();
    task_context.restart(&graph);

    // Now we can actually start the computation...

    // First, perform ITGR reduction.
    let (universe, active_variables) = interleaved_transition_guided_reduction(
        &task_context,
        &graph,
        graph.mk_unit_colored_vertices(),
    );

    println!(
        "Remaining after reduction: {}",
        universe.approx_cardinality()
    );

    // Then run Xie-Beerel to actually detect the components.
    xie_beerel_attractors(
        &task_context,
        &graph,
        &universe,
        &active_variables,
        |component| {
            println!("Found attractor... {}", component.approx_cardinality());
            println!("Remaining: {}", task_context.get_percent_string());
            println!(
                "Unique states: {}",
                component.vertices().approx_cardinality()
            );
            println!("Unique colors: {}", component.colors().approx_cardinality());
            classifier.add_component(component, &graph);
        },
    );

    classifier.print();

    let end = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_millis();

    let elapsed = end - start;

    println!(
        "Analysis completed. Classes: {}",
        classifier.export_result().len()
    );
    println!("Elapsed time: {}s", (elapsed as f64) / 1000.0);
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
            inlined_rg
                .add_regulation(old_regulator, old_target, false, reg.get_monotonicity())
                .unwrap();
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
        inlined_bn
            .add_string_update_function(name.as_str(), old_function.to_string(&bn).as_str())
            .unwrap();
    }

    inlined_bn
}
