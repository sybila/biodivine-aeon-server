use biodivine_aeon_server::scc::algo_symbolic_components::components_2;
use biodivine_aeon_server::scc::{Classifier, ProgressTracker};
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

    let graph = SymbolicAsyncGraph::new(model).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrisations: {}",
        graph.unit_colors().approx_cardinality()
    );
    println!(
        "State space: {}",
        graph.unit_vertices().approx_cardinality()
    );

    let classifier = Classifier::new(&graph);
    let progress = ProgressTracker::new(&graph);
    components_2(
        &graph,
        /*&progress, &AtomicBool::new(false), */
        |component| {
            println!("Found attractor...");
            println!("Remaining: {}", progress.get_percent_string());
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
