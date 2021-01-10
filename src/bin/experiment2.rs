use biodivine_aeon_server::scc::algo_symbolic_components::components_2;
use biodivine_aeon_server::scc::{Classifier, ProgressTracker, Behaviour};
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::{BooleanNetwork, VariableId, FnUpdate};
use std::convert::TryFrom;
use std::io::Read;
use rand::prelude::StdRng;
use rand::{SeedableRng, Rng};
use std::process::exit;

fn pick_valuation(inputs: &[VariableId], rng: &mut StdRng) -> Vec<(VariableId, bool)> {
    inputs.iter().map(|v| (*v, rng.gen_bool(0.5))).collect()
}

fn apply_network_inputs(network: &BooleanNetwork, valuation: &[(VariableId, bool)]) -> BooleanNetwork {
    let mut result = BooleanNetwork::new(network.as_graph().clone());
    for (v, b) in valuation {
        result.add_update_function(*v, FnUpdate::Const(*b)).unwrap();
    }
    for v in result.variables() {
        if result.get_update_function(v).is_none() {
            result.add_update_function(v, network.get_update_function(v).as_ref().unwrap().clone()).unwrap();
        }
    }
    return result;
}

fn main() {
    let mut random = StdRng::seed_from_u64(1234567890);
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    let names: Vec<_> = model
        .variables()
        .map(|id| model.get_variable_name(id))
        .collect();
    println!("Model loaded...");
    println!("{} variables: {:?}", model.num_vars(), names);

    let inputs: Vec<_> = model.variables().filter(|v| model.regulators(*v).len() == 0).collect();
    let mut i = 0;
    while i < 100 {
        let valuation = pick_valuation(&inputs, &mut random);
        //let valuation = inputs.iter().map(|v| (*v, true)).collect::<Vec<_>>();
        let model = apply_network_inputs(&model, &valuation);
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
                println!("Found attractor... {}", component.approx_cardinality());
                println!("Remaining: {}", progress.get_percent_string());
                if component.approx_cardinality() > 4.0 {
                    exit(0);
                }
                classifier.add_component(component, &graph);
            },
        );

        classifier.print();

        println!(
            "Analysis completed. Classes: {}",
            classifier.export_result().len()
        );

        /*for (cls, colors) in classifier.export_result() {
            if cls.get_vector().contains(&Behaviour::Disorder) || cls.get_vector().contains(&Behaviour::Oscillation) {
                println!("=============================== WITNESS ===============================");
                println!("{}", graph.pick_witness(&colors).to_string());

                return;
            }
        }*/
        i += 1;
    }
}
