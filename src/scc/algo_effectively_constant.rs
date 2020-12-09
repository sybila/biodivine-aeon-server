use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;

pub fn remove_effectively_constant_states_2(
    graph: &SymbolicAsyncGraph,
    set: GraphColoredVertices,
) -> GraphColoredVertices {
    println!("Remove effectively constant states.");
    let mut universe = set;
    for variable in graph.network().graph().variable_ids() {
        universe = cut_variable(graph, variable, universe);
    }
    return universe;
}

fn cut_variable(
    graph: &SymbolicAsyncGraph,
    variable: VariableId,
    universe: GraphColoredVertices,
) -> GraphColoredVertices {
    let mut scc_candidate = universe.clone();
    let mut not_attractor = graph.empty_vertices().clone();

    // First, find all states from which the variable cannot jump again and mark them as not attractor.
    // Meanwhile, compute a candidate for the SCCs of all variable jumps.
    loop {
        let vertices_where_var_can_jump = graph.has_any_post(variable, &scc_candidate);
        let vertices_where_var_jumped = graph.any_post(variable, &scc_candidate);
        let reachable_after_jump =
            reach_fwd_excluding(graph, &vertices_where_var_jumped, &scc_candidate, variable);
        let can_jump_again = reachable_after_jump.intersect(&vertices_where_var_can_jump);
        let will_never_jump_again = vertices_where_var_can_jump.minus(&can_jump_again);
        scc_candidate = reachable_after_jump;
        if will_never_jump_again.is_empty() {
            break;
        }
        not_attractor = not_attractor.union(&will_never_jump_again);
        println!(
            "{:?} will never jump again: {}",
            variable,
            will_never_jump_again.cardinality()
        );
    }

    // Now finish the SCCs
    let vertices_where_var_can_jump = graph.has_any_post(variable, &scc_candidate);
    let scc = reach_bwd_excluding(
        graph,
        &vertices_where_var_can_jump,
        &scc_candidate,
        variable,
    );
    let not_scc = graph.unit_vertices().minus(&scc);
    for other_variable in graph.network().graph().variable_ids() {
        let can_jump_out = graph.any_pre(other_variable, &not_scc).intersect(&scc);
        if !can_jump_out.is_empty() {
            println!("Can jump out: {}", can_jump_out.cardinality());
            not_attractor = not_attractor.union(&can_jump_out);
        }
    }

    let to_remove = reach_bwd_excluding(graph, &not_attractor, &universe, variable);
    println!(
        "Eliminated: {}/{}",
        to_remove.cardinality(),
        universe.cardinality()
    );
    return universe.minus(&to_remove);
}

/// This routine removes vertices which can never appear in an attractor by detecting parameter values
/// for which the variable jumps only in one direction.
///
/// If such one-directional jump is detected, then all states that can reach it are naturally
/// not in any attractor (since in an attractor, that jump would have to be reversed eventually).
///
/// Note that this does not mean the variable has to strictly always jump - that is why we need the
/// backward reachability to detect states that can actually achieve irreversible jump.
pub fn remove_effectively_constant_states(
    graph: &SymbolicAsyncGraph,
    set: GraphColoredVertices,
) -> GraphColoredVertices {
    println!("Remove effectively constant states.");
    let original_size = set.cardinality();
    let mut universe = set;
    let mut stop = false;
    while !stop {
        stop = true;
        let mut to_remove = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            let vertices_where_var_can_jump = graph.has_post(variable, &universe);
            let vertices_where_var_jumped = graph.post(variable, &universe, &universe);
            let reachable_after_jump =
                reach_fwd_excluding(graph, &vertices_where_var_jumped, &universe, variable);
            let can_jump_again = reachable_after_jump.intersect(&vertices_where_var_can_jump);
            let will_never_jump_again = vertices_where_var_can_jump.minus(&can_jump_again);
            if !will_never_jump_again.is_empty() {
                println!(
                    "{:?} will never jump again: {}",
                    variable,
                    will_never_jump_again.cardinality()
                );
            }
            if !will_never_jump_again.is_empty() {
                stop = false;
                let to_remove_for_var =
                    reach_bwd_excluding(graph, &will_never_jump_again, &universe, variable);
                to_remove = to_remove.union(&to_remove_for_var);
                //universe = universe.minus(&to_remove);
                println!(
                    "Eliminated {}/{} {:+e}%",
                    to_remove.cardinality(),
                    universe.cardinality(),
                    (to_remove.cardinality() / universe.cardinality()) * 100.0
                );
            }
        }
        universe = universe.minus(&to_remove);
        println!(
            "Universe now has {} nodes.",
            universe.clone().into_bdd().size()
        );
    }
    println!(
        "Removed {}/{} {:+e}%; {} nodes.",
        universe.cardinality(),
        original_size,
        (universe.cardinality() / original_size) * 100.0,
        universe.clone().into_bdd().size()
    );
    return universe;
}

pub fn reach_fwd_excluding(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    exclude: VariableId,
) -> GraphColoredVertices {
    let mut result = initial.clone();
    //println!("Reach fwd excluding {:?}...", exclude);
    loop {
        /*println!("{}/{} ({:+e}%, nodes result({}))",
                 result.cardinality(),
                 guard.cardinality(),
                 (result.cardinality()/guard.cardinality()) * 100.0,
                 result.clone().into_bdd().size()
        );*/
        let mut successors = graph.empty_vertices().clone();
        // iter over variables from the back
        for variable in graph.network().graph().variable_ids().rev() {
            if variable == exclude {
                continue;
            }
            let mut s = graph.post(variable, &result, guard);
            while !s.is_empty() {
                successors = successors.union(&s);
                s = graph.post(variable, &s, guard).minus(&successors);
            }
            //print!("...{:?} -> {}...", variable, s.into_bdd().size());
            //io::stdout().flush().unwrap();
        }
        //print!(" || {}", successors.clone().into_bdd().size());
        //println!();
        successors = successors.minus(&result);
        if successors.is_empty() {
            break;
        }
        result = result.union(&successors);
    }

    return result;
}

pub fn reach_bwd_excluding(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    exclude: VariableId,
) -> GraphColoredVertices {
    let mut result = initial.clone();
    //println!("Reach bwd excluding {:?}...", exclude);
    loop {
        /*println!("{}/{} ({:+e}%, nodes result({}))",
                 result.cardinality(),
                 guard.cardinality(),
                 (result.cardinality()/guard.cardinality()) * 100.0,
                 result.clone().into_bdd().size()
        );*/
        let mut predecessors = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids().rev() {
            if variable == exclude {
                continue;
            }
            let mut s = graph.pre(variable, &result, guard);
            while !s.is_empty() {
                predecessors = predecessors.union(&s);
                s = graph.pre(variable, &s, guard).minus(&predecessors);
            }
            //print!("...{:?} -> {}...", variable, s.into_bdd().size());
            //io::stdout().flush().unwrap();
        }
        //print!(" || {}", predecessors.clone().into_bdd().size());
        //println!();
        predecessors = predecessors.minus(&result);
        if predecessors.is_empty() {
            break;
        }
        result = result.union(&predecessors);
    }

    return result;
}
