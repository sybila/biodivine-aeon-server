use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;
use biodivine_aeon_server::scc::algo_xie_beerel::xie_beerel_attractors;
use biodivine_aeon_server::scc::Classifier;
use biodivine_aeon_server::GraphTaskContext;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
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
    let names: Vec<_> = model
        .variables()
        .map(|id| model.get_variable_name(id))
        .collect();
    println!("Model loaded...");
    println!("{} variables: {:?}", model.num_vars(), names);

    let graph = SymbolicAsyncGraph::new(&model).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrization set: {}",
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
