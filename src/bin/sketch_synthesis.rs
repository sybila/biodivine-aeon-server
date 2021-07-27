use biodivine_lib_param_bn::{BooleanNetwork, VariableId};
use std::io::Read;
use std::convert::TryFrom;
use rand::prelude::*;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_aeon_server::scc::Classifier;
use biodivine_aeon_server::GraphTaskContext;
use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;
use biodivine_aeon_server::scc::algo_xie_beerel::xie_beerel_attractors;
use biodivine_lib_param_bn::biodivine_std::traits::Set;

fn read_aeon_from_stdin() -> BooleanNetwork {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    BooleanNetwork::try_from(buffer.as_str()).unwrap()
}

fn inputs(model: &BooleanNetwork) -> Vec<VariableId> {
    model.variables()
        .filter(|v| model.regulators(*v).is_empty())
        .collect()
}

fn pick_observable_variables(random: &mut StdRng, model: &BooleanNetwork, p_observable: f64) -> Vec<VariableId> {
    model.variables()
        .filter(|v| !model.regulators(*v).is_empty())
        .filter(|_| random.gen_bool(p_observable))
        .collect()
}

fn make_sketch(model: &BooleanNetwork) -> BooleanNetwork {
    let mut new_network = BooleanNetwork::new(model.as_graph().clone());
    for v in model.variables() {
        let regs = model.regulators(v).len();
        if regs != 0 && regs != 2 { // No need to rewrite 1, that one is usually deterministic
            let function = model.get_update_function(v).as_ref().unwrap().clone();
            new_network.add_update_function(v, function).unwrap();
        }
    }

    new_network
}

fn get_all_attractors(graph: &SymbolicAsyncGraph) -> Vec<GraphColoredVertices> {
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
            classifier.add_component(component, &graph);
        },
    );

    let components = classifier.export_components();
    components.into_iter().map(|(s, _)| s).collect()
}

fn main() {
    let mut random = StdRng::seed_from_u64(123456789);
    let original_model = read_aeon_from_stdin();
    let original_graph = SymbolicAsyncGraph::new(original_model.clone()).unwrap();
    let observable_variables = pick_observable_variables(&mut random, &original_model, 0.4);

    let original_attractor = get_all_attractors(&original_graph)
        .into_iter()
        .next()
        .unwrap();

    for v in observable_variables.clone() {
        let v_set = original_graph.fix_network_variable(v, true);
        let v_not_set = original_graph.fix_network_variable(v, false);

        let v_states = original_attractor.intersect(&v_set);
        let v_not_states = original_attractor.intersect(&v_not_set);

        if !v_states.is_empty() && !v_not_states.is_empty() {
            println!("Variable {} is *unstable*", original_model.get_variable_name(v));
        } else if !v_states.is_empty() {
            println!("Variable {} is *true*", original_model.get_variable_name(v));
        } else {
            println!("Variable {} is *false*", original_model.get_variable_name(v));
        }
    }

    let sketch = make_sketch(&original_model);
    let sketch_graph = SymbolicAsyncGraph::new(sketch.clone()).unwrap();
    let all_bdd_vars = sketch_graph.symbolic_context().bdd_variable_set().num_vars();
    println!("Model variables: {}; Observable: {};", original_model.num_vars(), observable_variables.len());
    println!("Inputs: {};", inputs(&original_model).len());
    println!("Parameters in sketch: {};", usize::from(all_bdd_vars) - original_model.num_vars());
    println!("Valid parametrisations: {};", sketch_graph.unit_colors().approx_cardinality());

    /*for sketch_attractor in get_all_attractors(&sketch_graph) {
        println!("Sketch attractor (full): {}", sketch_attractor.approx_cardinality());

        let mut colors = sketch_attractor.colors();
        for v in observable_variables.clone() {
            let v_set_original = original_graph.fix_network_variable(v, true);
            let v_not_set_original = original_graph.fix_network_variable(v, false);

            let v_set_sketch = sketch_graph.fix_network_variable(v, true);
            let v_not_set_sketch = sketch_graph.fix_network_variable(v, false);

            let v_states_original = original_attractor.intersect(&v_set_original);
            let v_not_states_original = original_attractor.intersect(&v_not_set_original);

            let v_states_sketch = sketch_attractor.intersect(&v_set_sketch);
            let v_not_states_sketch = sketch_attractor.intersect(&v_not_set_sketch);

            if !v_states_original.is_empty() && !v_not_states_original.is_empty() {
                let in_both = v_states_sketch.colors().intersect(&v_not_states_sketch.colors());
                colors = colors.intersect(&in_both);
                println!("Variable {} is *unstable*; Remaining {}", original_model.get_variable_name(v), colors.approx_cardinality());
            } else if !v_states_original.is_empty() {
                let in_true = v_states_sketch.colors().minus(&v_not_states_sketch.colors());
                colors = colors.intersect(&in_true);
                println!("Variable {} is *true*; Remaining {}", original_model.get_variable_name(v), colors.approx_cardinality());
            } else {
                let in_false = v_not_states_sketch.colors().minus(&v_states_sketch.colors());
                colors = colors.intersect(&in_false);
                println!("Variable {} is *false*; Remaining {}", original_model.get_variable_name(v), colors.approx_cardinality());
            }
        }

        println!("Valid sketches: {}", colors.approx_cardinality());
    }*/
}