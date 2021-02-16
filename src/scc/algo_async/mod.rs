use crate::scc::algo_effectively_constant::{
    reach_saturated_bwd_excluding, reach_saturated_fwd_excluding,
};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;
use std::io::Write;

mod _impl_discard_bottom_basin;
mod _impl_discard_never_fires_again;
mod _impl_find_extended_component;
mod _impl_fwd_bwd_process;
mod _impl_graph_scheduler;
mod _impl_reach_after_var_can_fire;
mod _impl_reach_after_var_fired;

pub fn remove_effectively_constant_with_scheduler(
    graph: &SymbolicAsyncGraph,
    set: GraphColoredVertices,
) -> (Vec<GraphColoredVertices>, Vec<VariableId>) {
    let mut scheduler = GraphScheduler::mk(graph, &set);
    for variable in graph.network().variables() {
        //scheduler.spawn(Box::new(ReachAfterVarFired::mk(variable, &scheduler, graph)));
        scheduler.spawn(Box::new(ReachAfterVarCanFire::mk(
            variable, &scheduler, graph,
        )));
    }

    let mut iteration: usize = 0;
    while !scheduler.step(graph) {
        iteration += 1;
        if iteration % 100 == 0 {
            print!("{}|", iteration / 100);
            std::io::stdout().flush().unwrap();
        }
    }

    /*println!("===================== Phase 1 complete! ==============================");

    for variable in graph.network().variables() {
        scheduler.spawn(Box::new(ReachAfterVarFired::mk(variable, &scheduler, graph)));
        //scheduler.spawn(Box::new(ReachAfterVarCanFire::mk(variable, &scheduler, graph)));
    }

    let mut iteration: usize = 0;
    while !scheduler.step(graph) {
        iteration += 1;
        if iteration % 100 == 0 {
            print!("{}|", iteration / 100);
            std::io::stdout().flush().unwrap();
        }
    }*/

    let (universe, variables) = scheduler.finalize();
    return (vec![universe], variables);

    /*let mut conditions = Vec::new();
    for v in graph.network().variables() {
        conditions.push(graph.fix_network_variable(v, true));
        conditions.push(graph.fix_network_variable(v, false));
    }

    let c_count = conditions.len();
    conditions = conditions.iter().enumerate().filter(|(i, c)| {
        let is_useless = check_useless(graph, &universe, c);
        println!("Is {}/{} useless? {}", i, c_count, is_useless);
        !is_useless
    }).map(|(_, c)| c).cloned().collect();

    let mut universes = Vec::new();
    let mut todo = vec![(universe, conditions)];

    while let Some((universe, mut conditions)) = todo.pop() {
        if let Some(condition) = conditions.pop() {
            println!("Universe {}, {} conditions remaining.", universe.approx_cardinality(), conditions.len());
            let condition = condition.intersect(&universe);
            if condition.is_empty() || condition == universe {
                println!("No effect on {}.", universe.approx_cardinality());
                todo.push((universe, conditions));
                continue;
            }
            let condition_fwd = reach_saturated_fwd_excluding(
                graph,
                &condition,
                &universe,
                &variables
            );
            let condition_component = reach_saturated_bwd_excluding(
                graph,
                &condition,
                &condition_fwd,
                &variables
            );
            if condition_component == universe {
                println!("No effect on {}.", universe.approx_cardinality());
                todo.push((universe, conditions));
                continue;
            }
            let below_condition = condition_fwd.minus(&condition_component);
            let condition_component_basin = reach_saturated_bwd_excluding(
                graph,
                &condition_component,
                &universe,
                &variables
            );
            let below_condition_basin = reach_saturated_bwd_excluding(
                graph,
                &below_condition,
                &universe,
                &variables
            );
            let t1 = universe.minus(&below_condition_basin).minus(&condition_component_basin);
            let t2 = below_condition;
            let t3 = condition_component.minus(&below_condition_basin);
            println!("Split {} into {}/{}/{}", universe.approx_cardinality(), t1.approx_cardinality(), t2.approx_cardinality(), t3.approx_cardinality());
            if !t1.is_empty() {
                let c_count = conditions.len();
                let conditions = conditions.iter().enumerate().filter(|(i, c)| {
                    let is_useless = check_useless(graph, &t1, c);
                    println!("Is {}/{} useless? {}", i, c_count, is_useless);
                    !is_useless
                }).map(|(_, c)| c).cloned().collect();
                todo.push((t1, conditions));
            }
            if !t2.is_empty() {
                let c_count = conditions.len();
                let conditions = conditions.iter().enumerate().filter(|(i, c)| {
                    let is_useless = check_useless(graph, &t2, c);
                    println!("Is {}/{} useless? {}", i, c_count, is_useless);
                    !is_useless
                }).map(|(_, c)| c).cloned().collect();
                todo.push((t2, conditions));
            }
            if !t3.is_empty() {
                let c_count = conditions.len();
                let conditions = conditions.iter().enumerate().filter(|(i, c)| {
                    let is_useless = check_useless(graph, &t3, c);
                    println!("Is {}/{} useless? {}", i, c_count, is_useless);
                    !is_useless
                }).map(|(_, c)| c).cloned().collect();
                todo.push((t3, conditions));
            }
        } else {
            universes.push(universe);
            println!("Found candidate number {}.", universes.len());
        }
    }

    universes.sort_by(|a, b| a.approx_cardinality().partial_cmp(&b.approx_cardinality()).unwrap());

    (universes, variables)*/
}

fn _check_useless(
    graph: &SymbolicAsyncGraph,
    universe: &GraphColoredVertices,
    condition: &GraphColoredVertices,
) -> bool {
    let variables = graph.network().variables().collect();
    let condition = condition.intersect(universe);
    if condition.is_empty() || condition == *universe {
        return true;
    }
    let condition_fwd = reach_saturated_fwd_excluding(graph, &condition, &universe, &variables);
    let condition_component =
        reach_saturated_bwd_excluding(graph, &condition, &condition_fwd, &variables);
    if condition_component == *universe {
        return true;
    }
    let below_condition = condition_fwd.minus(&condition_component);
    let condition_component_basin =
        reach_saturated_bwd_excluding(graph, &condition_component, &universe, &variables);
    let below_condition_basin =
        reach_saturated_bwd_excluding(graph, &below_condition, &universe, &variables);
    let t1 = universe
        .minus(&below_condition_basin)
        .minus(&condition_component_basin);
    let t2 = below_condition;
    let t3 = condition_component.minus(&below_condition_basin);
    (t1.is_empty() && t2.is_empty())
        | (t1.is_empty() && t3.is_empty())
        | (t2.is_empty() && t3.is_empty())
}

/// Process scheduler that will execute spawned processes. It also maintains the current "universe"
/// of unfinished nodes.
struct GraphScheduler {
    universe: GraphColoredVertices,
    active_variables: Vec<VariableId>,
    active_processes: Vec<Box<dyn Process>>,
    discard_next: Option<GraphColoredVertices>,
}

/// Process is a unit of work which can be executed using a `GraphScheduler`.
trait Process {
    /// Perform one step of this process. If `true`, the process is done and can be destroyed.
    ///
    /// Note that by contract you should not call `step` after it has returned `true`. It should
    /// be mostly ok for many processes, but it can cause unexpected side-effects, like spawning
    /// succeeding processes multiple times.
    fn step(&mut self, scheduler: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool;

    /// The weight of a process - the approximate amount of resources the process has alocated.
    fn weight(&self) -> usize;

    /// Mark the given vertex set as done - can be removed from consideration entirely.
    /// Note: The set is typically expected to be bwd-closed.
    fn discard(&mut self, set: &GraphColoredVertices);

    fn name(&self) -> &str;
}

/// Basic forward reachability process. When it is finished, the `reach_set` contains all
/// vertices in `universe` forward reachable from the `initial` set using
/// transitions with the given `variables`.
struct FwdProcess {
    variables: Vec<VariableId>,
    universe: GraphColoredVertices,
    fwd_set: GraphColoredVertices,
    active_variable: usize,
    name: String,
}

/// Basic forward reachability process. When it is finished, the `reach_set` contains all
/// vertices in `universe` backward reachable from the `initial` set using
/// transitions with the given `variables`.
struct BwdProcess {
    variables: Vec<VariableId>,
    universe: GraphColoredVertices,
    bwd_set: GraphColoredVertices,
    active_variable: usize,
    name: String,
}

/// First step of the reduction algorithm. Computes the vertices reachable *after* firing a
/// certain variable.
struct ReachAfterVarFired {
    variable: VariableId,
    fwd: FwdProcess,
    name: String,
}

struct ReachAfterVarCanFire {
    variable: VariableId,
    fwd: FwdProcess,
    name: String,
}

/// After a node which can do transition that is never fired again is found, this process
/// computes all nodes eliminated by this finding and discards them. Then it restarts the
/// `ReachAfterVarFired` process.
struct DiscardNeverFiresAgain {
    variable: VariableId,
    bwd: BwdProcess,
    name: String,
}

/// After we have forward reachability from any set of vertices, we can compute the extended
/// component by also running backward reachability in this region. Afterwards, we can determine
/// if there is any bottom region under this extended component. If so, basin of that bottom
/// region can also be removed (by the next spawned process).
struct FindExtendedComponent {
    fwd_set: GraphColoredVertices,
    bwd: BwdProcess,
    name: String,
}

/// Compute basin of a proved bottom region and then discard it.
struct DiscardBottomBasin {
    bottom_region: GraphColoredVertices,
    bwd: BwdProcess,
    name: String,
}
