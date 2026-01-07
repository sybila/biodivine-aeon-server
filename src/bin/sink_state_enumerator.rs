use biodivine_aeon_server::scc::{Behaviour, Classifier};
use biodivine_aeon_server::GraphTaskContext;
use biodivine_algo_bdd_scc::attractor::{
    AttractorConfig, InterleavedTransitionGuidedReduction, ItgrState, XieBeerelAttractors,
};
use biodivine_lib_param_bn::biodivine_std::bitvector::BitVector;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use computation_process::{Computable, Stateful};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::io::Read;
use std::time::SystemTime;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let start = SystemTime::now();

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
        println!("Remaining: {}", task_context.get_percent_string());
        println!(
            "Unique states: {}",
            component.vertices().approx_cardinality()
        );
        println!("Unique colors: {}", component.colors().approx_cardinality());
        classifier.add_component(component, &graph);
    }

    classifier.print();

    println!(
        "Analysis completed. Classes: {}",
        classifier.export_result().len()
    );
    println!("Elapsed time: {}s", start.elapsed().unwrap().as_secs());

    let mut all_sinks = graph.mk_empty_colored_vertices();
    for (attractor, behaviours) in classifier.export_components() {
        if let Some(sink_colors) = behaviours.get(&Behaviour::Stability) {
            all_sinks = all_sinks.union(&attractor.intersect_colors(sink_colors));
        }
    }

    println!(
        "Explicit sinks ({} states):",
        all_sinks.vertices().approx_cardinality()
    );

    for vertex in all_sinks.vertices().materialize().iter() {
        let values = model
            .variables()
            .enumerate()
            .map(|(i, v)| (model.get_variable_name(v).clone(), vertex.get(i)))
            .collect::<Vec<_>>();
        for (name, value) in values {
            print!("{}: {}; ", name, value);
        }
        let sink = graph.vertex(&vertex);
        println!(
            "{} instance(s).",
            sink.intersect(&all_sinks).approx_cardinality()
        );
    }
}
