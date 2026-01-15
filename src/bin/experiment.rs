use biodivine_aeon_server::GraphTaskContext;
use biodivine_aeon_server::scc::Classifier;
use biodivine_algo_bdd_scc::attractor::{
    AttractorConfig, InterleavedTransitionGuidedReduction, ItgrState, XieBeerelAttractors,
};
use biodivine_lib_param_bn::BooleanNetwork;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use computation_process::{Computable, Stateful};
use std::collections::BTreeSet;
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
    task_context.init_progress(&graph);

    // Now we can actually start the computation...

    // First, perform ITGR reduction.
    let state = ItgrState::new(&graph, graph.unit_colored_vertices());
    let mut itgr = InterleavedTransitionGuidedReduction::configure(&graph, state);
    let universe = itgr.compute().expect("Cancellation disabled.");
    let active_variables = itgr.state().active_variables().collect::<BTreeSet<_>>();

    // Then run Xie-Beerel to actually detect the components.
    let mut config = AttractorConfig::new(graph.clone());
    config.active_variables = active_variables;
    let attractors = XieBeerelAttractors::configure(config, universe);

    for component in attractors {
        let component = component.expect("Cancellation disabled.");
        println!("Found attractor... {}", component.approx_cardinality());
        println!("Remaining: {}", task_context.get_progress_string());
        println!(
            "Unique states: {}",
            component.vertices().approx_cardinality()
        );
        println!("Unique colors: {}", component.colors().approx_cardinality());
        classifier.add_component(component, &graph);
    }

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
