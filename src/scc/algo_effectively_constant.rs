use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};
use biodivine_lib_param_bn::VariableId;

/// This routine removes vertices which can never appear in an attractor by detecting parameter values
/// for which the variable jumps only in one direction.
///
/// If such one-directional jump is detected, then all states that can reach it are naturally
/// not in any attractor (since in an attractor, that jump would have to be reversed eventually).
///
/// Note that this does not mean the variable has to strictly always jump - that is why we need the
/// backward reachability to detect states that can actually achieve irreversible jump.
pub fn remove_effectively_constant_states(graph: &SymbolicAsyncGraph, set: GraphColoredVertices) -> GraphColoredVertices {
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
            let reachable_after_jump = reach_fwd_excluding(graph, &vertices_where_var_jumped, &universe, variable);
            let can_jump_again = reachable_after_jump.intersect(&vertices_where_var_can_jump);
            let will_never_jump_again = vertices_where_var_can_jump.minus(&can_jump_again);
            if !will_never_jump_again.is_empty() {
                println!("{:?} will never jump again: {}", variable, will_never_jump_again.cardinality());
            }
            if !will_never_jump_again.is_empty() {
                stop = false;
                let to_remove_for_var = reach_bwd_excluding(graph, &will_never_jump_again, &universe, variable);
                to_remove = to_remove.union(&to_remove_for_var);
                //universe = universe.minus(&to_remove);
                println!("Eliminated {}/{} {:+e}%", to_remove.cardinality(), universe.cardinality(), (to_remove.cardinality()/universe.cardinality()) * 100.0);
            }
        }
        universe = universe.minus(&to_remove);
        println!("Universe now has {} nodes.", universe.clone().into_bdd().size());
    }
    println!("Removed {}/{} {:+e}%; {} nodes.", universe.cardinality(), original_size, (universe.cardinality()/original_size) * 100.0, universe.clone().into_bdd().size());
    return universe;
}

pub fn reach_fwd_excluding(graph: &SymbolicAsyncGraph, initial: &GraphColoredVertices, guard: &GraphColoredVertices, exclude: VariableId) -> GraphColoredVertices {
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
                s = graph.post(variable, &s, guard);
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

pub fn reach_bwd_excluding(graph: &SymbolicAsyncGraph, initial: &GraphColoredVertices, guard: &GraphColoredVertices, exclude: VariableId) -> GraphColoredVertices {
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
                s = graph.pre(variable, &s, guard);
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