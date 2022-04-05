use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;
use biodivine_aeon_server::scc::algo_xie_beerel::xie_beerel_attractors;
use biodivine_aeon_server::scc::{Behaviour, Classifier};
use biodivine_aeon_server::GraphTaskContext;
use biodivine_lib_param_bn::biodivine_std::bitvector::BitVector;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
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

    let graph = SymbolicAsyncGraph::new(model.clone(), 0).unwrap();

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

    println!(
        "Analysis completed. Classes: {}",
        classifier.export_result().len()
    );
    println!("Elapsed time: {}s", start.elapsed().unwrap().as_secs());

    let mut all_sinks = graph.mk_empty_vertices();
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
