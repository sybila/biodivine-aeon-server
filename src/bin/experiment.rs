use biodivine_aeon_server::scc::algo_components::components;
use biodivine_aeon_server::scc::{Classifier, ProgressTracker};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use std::convert::TryFrom;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};
use biodivine_aeon_server::bdt::make_decision_tree;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_millis();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    let names: Vec<_> = model
        .graph()
        .variable_ids()
        .map(|id| model.graph().get_variable(id).clone())
        .collect();
    println!("Model loaded...");
    println!("{} variables: {:?}", model.graph().num_vars(), names);

    let graph = AsyncGraph::new(model.clone()).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrisations: {}",
        graph.unit_params().cardinality()
    );

    let classifier = Classifier::new(&graph);
    let progress = ProgressTracker::new(&graph);
    components(&graph, &progress, &AtomicBool::new(false), |component| {
        println!("Found attractor...");
        println!("Remaining: {}", progress.get_percent_string());
        classifier.add_component(component, &graph);
    });

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

    println!("Start learning tree...\n\n");
    make_decision_tree(&model, &classifier.export_result());
}
