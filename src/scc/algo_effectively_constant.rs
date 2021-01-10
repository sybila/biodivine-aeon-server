use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;
use std::io::Write;

struct VarProcess<'a> {
    graph: &'a SymbolicAsyncGraph,
    variable: VariableId,               // Variable examined by this process.
    active_variables: Vec<VariableId>,  // Variables used for reachability.
    active_variable: usize,             // Variable which will be used for next reachability step.
    universe: GraphColoredVertices,     // Total set of states that we need to explore.
    reach_fwd: GraphColoredVertices,     // So-far reached states.
    components: Option<GraphColoredVertices>,    // Once reach-fwd is done, this will contain the components
    can_reach_not_component: Option<GraphColoredVertices>, // Last reachability procedure
}

impl<'a> VarProcess<'a> {

    fn complexity(&self) -> usize {
        if let Some(a) = self.can_reach_not_component.as_ref() {
            a.as_bdd().size()
        } else if let Some(b) = self.components.as_ref() {
            b.as_bdd().size()
        } else {
            self.reach_fwd.as_bdd().size()
        }
    }

}

impl<'a> VarProcess<'a> {

    fn mk<'b, 'c>(
        graph: &'a SymbolicAsyncGraph,
        variable: VariableId,
        universe: &'b GraphColoredVertices,
        active_variables: &'c [VariableId]
    ) -> VarProcess<'a> {
        /*let active_variables: Vec<VariableId> = active_variables
            .iter()
            .cloned()
            .filter(|it| *it != variable)
            .collect();*/
        if active_variables.len() == 0 {
            panic!("No active variables."); // TODO: Should not panic.
        }
        VarProcess {
            graph, variable,
            universe: universe.clone(),
            active_variable: active_variables.len() - 1,
            reach_fwd: graph.var_can_post(variable, universe),
            active_variables: active_variables.to_vec(),
            components: None,
            can_reach_not_component: None
        }
    }

    fn make_step(&mut self) -> Option<GraphColoredVertices> {
        if let Some(can_reach_not_component) = self.can_reach_not_component.as_mut() {
            // Finally, this reachability computes the states that can reach the "lower" area of the graph
            let step_var = self.active_variables[self.active_variable];
            let pre = self.graph
                .var_pre(step_var, can_reach_not_component)
                .intersect(&self.universe)
                .minus(can_reach_not_component);

            if !pre.is_empty() {
                *can_reach_not_component = can_reach_not_component.union(&pre);
                self.active_variable = self.active_variables.len() - 1;
            } else {
                if self.active_variable == 0 {
                    // We are done - now we can actually compute the vertices that
                    let fwd_but_not_component = self.reach_fwd.minus(self.components.as_ref().unwrap());
                    return Some(can_reach_not_component.minus(&fwd_but_not_component))
                } else {
                    self.active_variable -= 1;
                }
            }
            None
        } else if let Some(components) = self.components.as_mut() {
            // This is reachability computes the actual SCCs that contain the pivot set
            let step_var = self.active_variables[self.active_variable];
            let pre = self.graph
                .var_pre(step_var, components)
                .intersect(&self.reach_fwd)
                .minus(components);

            if !pre.is_empty() {
                *components = components.union(&pre);
                self.active_variable = self.active_variables.len() - 1;
            } else {
                if self.active_variable == 0 {
                    // We are done - components are now complete - we can identify border vertices and start bwd reachability
                    let fwd_but_not_component = self.reach_fwd.minus(&components);
                    self.can_reach_not_component = Some(fwd_but_not_component);
                    self.active_variable = self.active_variables.len() - 1;
                } else {
                    self.active_variable -= 1;
                }
            }
            None
        } else {
            // This is basic fwd reachability from vertices that can perform a jump in a specific variable
            let step_var = self.active_variables[self.active_variable];
            let post = self.graph
                .var_post(step_var, &self.reach_fwd)
                .intersect(&self.universe)
                .minus(&self.reach_fwd);

            if !post.is_empty() {
                self.reach_fwd = self.reach_fwd.union(&post);
                self.active_variable = self.active_variables.len() - 1;
            } else {
                if self.active_variable == 0 {
                    // We are done - reach_fwd now has all values reachable after jump.
                    // We can start computing the actual components
                    self.components = Some(self.graph.var_can_post(self.variable, &self.universe));
                    self.active_variable = self.active_variables.len() - 1;
                } else {
                    self.active_variable -= 1;
                }
            }
            None
        }
    }

    fn restrict_universe(&mut self, to_remove: &GraphColoredVertices) {
        self.universe = self.universe.minus(to_remove);
        self.reach_fwd = self.reach_fwd.minus(to_remove);
        if let Some(comp) = self.components.as_mut() {
            *comp = comp.minus(to_remove);
        }
        if let Some(bwd) = self.can_reach_not_component.as_mut() {
            *bwd = bwd.minus(to_remove);
        }
    }

}

pub fn _remove_effectively_constant_states_lockstep(
    graph: &SymbolicAsyncGraph,
    set: GraphColoredVertices
) -> (GraphColoredVertices, Vec<VariableId>) {
    println!("Remove effectively constant states.");
    let original_size = set.approx_cardinality();
    let mut universe = set;
    let all_variables: Vec<VariableId> = graph.network().variables().collect();
    let mut variables: Vec<VariableId> = graph.network().variables().collect();
    let mut processes: Vec<Option<VarProcess>> = graph.network().variables().map(|v| {
        Some(VarProcess::mk(graph, v, &universe, &variables))
    }).collect();
    let mut iter: usize = 0;
    let mut i_p2: usize = 0;
    loop {
        let (i_p, _) = processes.iter().enumerate().min_by_key(|(_, p)| {
            p.as_ref().map(|p| p.complexity()).unwrap_or(usize::MAX)
        }).unwrap();
        let will_never_jump_again = processes[i_p]
            .as_mut()
            .and_then(|p| p.make_step());
        if let Some(to_remove) = will_never_jump_again {
            processes[i_p] = None;
            let i_var = all_variables[i_p];
            let remaining = processes.iter().filter(|p| p.is_some()).count();
            println!("Finished {}. Remaining {}.", i_p, remaining);
            /*let components = reach_saturated_bwd_excluding(
                graph,
                &graph.var_can_post(i_var, &universe),
                &reach_fwd,
                &variables
            );
            let fwd_but_not_component = reach_fwd.minus(&components);
            println!("Finished component - find backwards reachable.");
            let can_reach_not_component = reach_saturated_bwd_excluding(
                graph,
                &fwd_but_not_component,
                &universe,
                &variables
            );
            let to_remove = can_reach_not_component.minus(&fwd_but_not_component);*/
            println!("Remove: {}/{}", to_remove.approx_cardinality(), universe.approx_cardinality());
            if !to_remove.is_empty() {
                universe = universe.minus(&to_remove);
                processes.iter_mut().for_each(|p| {
                    if let Some(p) = p.as_mut() {
                        p.restrict_universe(&to_remove);
                    }
                });
            }

            let is_constant = graph
                .var_can_post(i_var, &universe).is_empty();
            if is_constant {
                // Constant variables are removed (and will never be restarted).
                variables = variables.into_iter().filter(|it| *it != i_var).collect();
                //if variables.len() == 1 { break; }  // TODO: this is a hack for models with a lot of constants
            }
        }
        /*if let Some(will_never_jump_again) = will_never_jump_again {
            processes[i_p] = None;  // "Stop" current process.
            if will_never_jump_again.is_empty() {
                // No more progress can be made for this variable at the moment.
                let is_constant = graph
                    .var_can_post(all_variables[i_p], &universe).is_empty();
                if is_constant {
                    // Constant variables are removed (and will never be restarted).
                    let variable = all_variables[i_p];
                    variables = variables.into_iter().filter(|it| *it != variable).collect();
                    if variables.len() == 1 { break; }  // TODO: this is a hack for models with a lot of constants
                }
                let remaining = processes.iter().filter(|it| it.is_some()).count();
                println!("\n Variable {:?} is finished for now (constant {}; active vars {}, remaining {}).", i_p, is_constant, variables.len(), remaining);
            } else {
                restart = true;
                // We have eliminated some state space!
                // Remove it from existing processes
                println!(
                    "\n Variable {:?} eliminated {}/{} states.",
                    i_p,
                    will_never_jump_again.approx_cardinality(),
                    universe.approx_cardinality()
                );
                universe = universe.minus(&will_never_jump_again);
                processes.iter_mut().for_each(|p| {
                    if let Some(p) = p.as_mut() {
                        p.restrict_universe(&will_never_jump_again);
                    }
                });
                processes[i_p] = Some(VarProcess::mk(graph, all_variables[i_p], &universe, &variables));
                // Restart processes that are not constant - they may need to be recomputed

            }
        }*/

        // Once everything is done, there should be no remaining process
        if processes.iter().all(|it| it.is_none()) {
            break;
            /*if restart {
                let mut total = 0;
                for (j_p, p) in processes.iter_mut().enumerate() {
                    if p.is_none() && variables.contains(&all_variables[j_p]) {
                        total += 1;
                        *p = Some(VarProcess::mk(graph, all_variables[j_p], &universe, &variables));
                    }
                }
                println!("Restarted {}.", total);
                restart = false;
            } else {
                break;
            }*/
        }
        // Move to next process...
        i_p2 += 1;
        if i_p2 == processes.len() {
            i_p2 = 0;
            iter += 1;
            print!("|{}", iter);
            std::io::stdout().flush().unwrap();
        }
    }

    println!("Final active variables: {}", variables.len());
    println!(
        "Removed {}/{} {:+e}%; {} nodes.",
        universe.approx_cardinality(),
        original_size,
        (universe.approx_cardinality() / original_size) * 100.0,
        universe.clone().into_bdd().size()
    );

    /*for v in &variables {
        let vertices_where_var_can_jump = graph.var_can_post(*v, &universe);
        let reachable_before_jump = reach_saturated_bwd_excluding(
            graph,
            &vertices_where_var_can_jump,
            &universe,
            &variables,
        );
        let reachable_after_jump = reach_saturated_fwd_excluding(
            graph,
            &vertices_where_var_can_jump,
            &universe,
            &variables,
        );
        let components = reachable_before_jump.intersect(&reachable_after_jump);
        let below = reachable_after_jump.minus(&components);
        let can_reach_below =
            reach_saturated_bwd_excluding(graph, &below, &universe, &variables).minus(&below);
        println!(
            "({:?}) Below: {} Can reach below: {}",
            v,
            below.approx_cardinality(),
            can_reach_below.approx_cardinality()
        );
        universe = universe.minus(&can_reach_below);
    }*/

    println!("Final active variables: {}", variables.len());
    println!(
        "Removed {}/{} {:+e}%; {} nodes.",
        universe.approx_cardinality(),
        original_size,
        (universe.approx_cardinality() / original_size) * 100.0,
        universe.clone().into_bdd().size()
    );
    return (universe, variables);
}

/// This routine removes vertices which can never appear in an attractor by detecting parameter values
/// for which the variable jumps only in one direction.
///
/// If such one-directional jump is detected, then all states that can reach it are naturally
/// not in any attractor (since in an attractor, that jump would have to be reversed eventually).
///
/// Note that this does not mean the variable has to strictly always jump - that is why we need the
/// backward reachability to detect states that can actually achieve irreversible jump.
pub fn _old_remove_effectively_constant_states(
    graph: &SymbolicAsyncGraph,
    set: GraphColoredVertices,
) -> (GraphColoredVertices, Vec<VariableId>) {
    println!("Remove effectively constant states.");
    let original_size = set.approx_cardinality();
    let mut universe = set;
    let mut stop = false;
    let mut variables: Vec<VariableId> = graph.network().variables().collect();
    while !stop {
        stop = true;
        let mut to_remove = graph.empty_vertices().clone();
        for variable in graph.network().variables() {
            let active_variables: Vec<VariableId> = variables
                .iter()
                .cloned()
                .filter(|it| *it != variable)
                .collect();
            let vertices_where_var_can_jump = graph.var_can_post(variable, &universe);
            let vertices_where_var_jumped = graph.var_post(variable, &universe);
            let reachable_after_jump = reach_saturated_fwd_excluding(
                graph,
                &vertices_where_var_jumped,
                &universe,
                &active_variables,
            );
            let will_never_jump_again = vertices_where_var_can_jump.minus(&reachable_after_jump);
            if !will_never_jump_again.is_empty() {
                stop = false;
                println!("({:?}) Will never jump again: {}", variable, will_never_jump_again.approx_cardinality());
                let to_remove_for_var = reach_saturated_bwd_excluding(
                    graph,
                    &will_never_jump_again,
                    &universe,
                    &active_variables,
                );
                to_remove = to_remove.union(&to_remove_for_var);
                //universe = universe.minus(&to_remove); THIS IS A BAD IDEA...
                /*println!(
                    "{:?} will never jump again: {}",
                    variable,
                    will_never_jump_again.approx_cardinality()
                );*/
                println!("({:?}) To remove: {}", variable, to_remove_for_var.approx_cardinality());
                /*println!(
                    "Eliminated {}/{} {:+e}%",
                    to_remove.approx_cardinality(),
                    universe.approx_cardinality(),
                    (to_remove.approx_cardinality() / universe.approx_cardinality()) * 100.0
                );*/
            }
        }
        universe = universe.minus(&to_remove);
        let original_vars = variables.len();
        variables = variables
            .into_iter()
            .filter(|v| !graph.var_can_post(*v, &universe).is_empty())
            .collect();
        println!(
            "Universe now has {} nodes and size {}. Eliminated {} variables.",
            universe.clone().into_bdd().size(),
            universe.approx_cardinality(),
            original_vars - variables.len()
        );
    }

    println!("Final active variables: {}", variables.len());
    println!(
        "Removed {}/{} {:+e}%; {} nodes.",
        universe.approx_cardinality(),
        original_size,
        (universe.approx_cardinality() / original_size) * 100.0,
        universe.clone().into_bdd().size()
    );

    for v in &variables {
        let vertices_where_var_can_jump = graph.var_can_post(*v, &universe);
        let reachable_before_jump = reach_saturated_bwd_excluding(
            graph,
            &vertices_where_var_can_jump,
            &universe,
            &variables,
        );
        let reachable_after_jump = reach_saturated_fwd_excluding(
            graph,
            &vertices_where_var_can_jump,
            &universe,
            &variables,
        );
        let components = reachable_before_jump.intersect(&reachable_after_jump);
        let below = reachable_after_jump.minus(&components);
        let can_reach_below =
            reach_saturated_bwd_excluding(graph, &below, &universe, &variables).minus(&below);
        println!(
            "({:?}) Below: {} Can reach below: {}",
            v,
            below.approx_cardinality(),
            can_reach_below.approx_cardinality()
        );
        universe = universe.minus(&can_reach_below);
    }

    println!("Final active variables: {}", variables.len());
    println!(
        "Removed {}/{} {:+e}%; {} nodes.",
        universe.approx_cardinality(),
        original_size,
        (universe.approx_cardinality() / original_size) * 100.0,
        universe.clone().into_bdd().size()
    );
    return (universe, variables);
}

pub fn reach_saturated_fwd_excluding(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    variables: &Vec<VariableId>,
) -> GraphColoredVertices {
    if variables.is_empty() {
        return initial.clone();
    }
    let mut result = initial.clone();
    let last_variable = variables.len() - 1;
    let mut active_variable = last_variable;
    loop {
        let variable = variables[active_variable];
        let post = graph
            .var_post(variable, &result)
            .intersect(guard)
            .minus(&result);
        result = result.union(&post);

        if !post.is_empty() {
            active_variable = last_variable;
        } else {
            if active_variable == 0 {
                break;
            } else {
                active_variable -= 1;
            }
        }
    }
    return result;
}

pub fn reach_saturated_bwd_excluding(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    variables: &Vec<VariableId>,
) -> GraphColoredVertices {
    if variables.is_empty() {
        return initial.clone();
    }
    let mut result = initial.clone();
    let last_variable = variables.len() - 1;
    let mut active_variable = last_variable;
    loop {
        let variable = variables[active_variable];
        let post = graph
            .var_pre(variable, &result)
            .intersect(guard)
            .minus(&result);

        if !post.is_empty() {
            result = result.union(&post);
            active_variable = last_variable;
        } else {
            if active_variable == 0 {
                break;
            } else {
                active_variable -= 1;
            }
        }
    }
    return result;
}
