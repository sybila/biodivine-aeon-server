use std::time::{SystemTime, UNIX_EPOCH};
use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};
use biodivine_aeon_server::GraphTaskContext;
use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;
use biodivine_aeon_server::scc::algo_xie_beerel::xie_beerel_attractors;
use std::io::Read;
use biodivine_lib_param_bn::BooleanNetwork;
use std::sync::Mutex;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_millis();

    println!("Loading model from .bnet file...");
    let model = BooleanNetwork::try_from_bnet(buffer.as_str()).unwrap();
    println!("Model loaded...");
    println!("!!! WARNING: .bnet support is currently experimental. Here is a list of variables and update functions parsed from the .bnet file. Please make sure they are parsed correctly in case of problems.");
    for v in model.variables() {
        if let Some(fun) = model.get_update_function(v) {
            println!("{} = {}", model.get_variable_name(v), fun.to_string(&model));
        } else {
            println!("{} = INPUT", model.get_variable_name(v));
        }
    }

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

    let task_context = GraphTaskContext::new();
    task_context.restart(&graph);

    // Now we can actually start the computation...

    // First, perform ITGR reduction.
    let (universe, active_variables) = interleaved_transition_guided_reduction(
        &task_context,
        &graph,
        graph.mk_unit_colored_vertices(),
    );
    //let (universe, active_variables) = (graph.mk_unit_colored_vertices(), graph.as_network().variables().collect::<Vec<_>>());

    let attractors: Mutex<Vec<GraphColoredVertices>> = Mutex::new(Vec::new());
    // Then run Xie-Beerel to actually detect the components.
    xie_beerel_attractors(
        &task_context,
        &graph,
        &universe,
        &active_variables,
        |component| {
            let mut attractors = attractors.lock().unwrap();
            attractors.push(component);
        },
    );

    let end = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_millis();

    let elapsed = end - start;

    let attractors = attractors.lock().unwrap();
    println!("Analysis completed. Unique attractors: {}", attractors.len());

    for (i, attr) in attractors.iter().enumerate() {
        println!("Attractor #{}:", i+1);
        println!("Unique states: {}; Parametrisations: {}",
                    attr.vertices().approx_cardinality(),
                    attr.colors().approx_cardinality()
        );
    }
    println!("Elapsed time: {}s", (elapsed as f64) / 1000.0);
}