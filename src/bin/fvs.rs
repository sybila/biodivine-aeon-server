use biodivine_lib_param_bn::{BooleanNetwork, VariableId};
use std::io::Read;
use std::collections::{HashSet, HashMap, VecDeque};
use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction_with_variables;
use biodivine_aeon_server::GraphTaskContext;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use std::convert::TryFrom;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_aeon_server::scc::algo_saturated_reachability::{reach_fwd, reach_bwd};

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    let model = BooleanNetwork::try_from_bnet(buffer.as_str()).unwrap();

    let mut exclude: HashSet<VariableId> = HashSet::new();

    while let Some(to_exclude) = pick_variable_to_exclude(&model, &exclude) {
        println!("Excluding {}.", model.get_variable_name(to_exclude));
        exclude.insert(to_exclude);
    }

    println!("Excluded: {}.", exclude.len());

    let ctx = GraphTaskContext::new();
    let graph = SymbolicAsyncGraph::new(model.clone()).unwrap();
    let variables: Vec<VariableId> = model.variables().filter(|v| !exclude.contains(v)).collect();

/*
    let remaining_vars: Vec<VariableId> = Vec::new();
    let mut reduced = graph.mk_unit_colored_vertices();

    for v in variables {
        let can_post = graph.var_can_post(v, graph.unit_colored_vertices());
        reduced = reduced.minus(&can_post);
        println!("Eliminated {}, result uses {} nodes and contains {} states.", model.get_variable_name(v), reduced.as_bdd().size(), reduced.approx_cardinality());
    }
*/
    ctx.restart(&graph);
    let (reduced, remaining_vars) = interleaved_transition_guided_reduction_with_variables(
        &ctx,
        &graph,
        graph.mk_unit_colored_vertices(),
        variables,
        graph.unit_colored_vertices()
    );

    println!("Reduced set size: {} represented with {} nodes.", reduced.approx_cardinality(), reduced.as_bdd().size());
    println!("Remaining variables: {}", remaining_vars.len());

    let extra_var = *exclude.iter().next().unwrap();
    let all_variables: Vec<VariableId> = model.variables().filter(|v| !exclude.contains(v) || *v == extra_var).collect();
    let fwd = reach_fwd(&ctx, &graph, &reduced, graph.unit_colored_vertices(), &all_variables);

    println!("Forward set: {} {}/{}", fwd.as_bdd().size(), fwd.approx_cardinality(), graph.unit_colored_vertices().approx_cardinality());
}

fn pick_variable_to_exclude(model: &BooleanNetwork, exclude: &HashSet<VariableId>) -> Option<VariableId> {
    let mut cycle_length = Vec::new();
    for v in model.variables() {
        if exclude.contains(&v) {
            continue;
        }
        let length = find_shortest_cycle(model, v, exclude);
        if length > 0 {
            cycle_length.push((v, length));
        }
    }

    if cycle_length.is_empty() {
        return None;    // Graph is acyclic.
    }

    // Sort by cycle length
    cycle_length.sort_by(|(_, l1), (_, l2)| l1.cmp(l2));

    // Remove non-minimal cycles
    let min_length = cycle_length[0].1;
    cycle_length = cycle_length.into_iter().filter(|(_, l)| *l == min_length).collect();

    // Sort by degree
    cycle_length.sort_by(|(v1, _), (v2, _)| {
        let d1 = model.targets(*v1).len() + model.regulators(*v1).len();
        let d2 = model.targets(*v2).len() + model.regulators(*v2).len();
        d1.cmp(&d2).reverse()
    });

    return Some(cycle_length[0].0);
}

/// Compute the shortest cycle that contains the given [variable], without considering the
/// [exclude] variables.
///
/// If the variable is not on a cycle, return -1.
fn find_shortest_cycle(model: &BooleanNetwork, variable: VariableId, exclude: &HashSet<VariableId>) -> i32 {
    let mut done = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(variable);
    let mut distance: HashMap<VariableId, i32> = HashMap::new();
    distance.insert(variable, 0);

    while let Some(node) = queue.pop_front() {
        for successor in model.as_graph().targets(node) {
            if successor == variable {
                return distance[&node] + 1;
            }

            if exclude.contains(&successor) || done.contains(&successor) {
                continue;
            }

            distance.insert(successor, distance[&node] + 1);
            queue.push_back(successor);
            done.insert(successor);
        }
    }


    return -1;
}