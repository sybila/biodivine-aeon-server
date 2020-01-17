use super::algo_reach::{guarded_reach, reach};
use super::StateSet;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_std::param_graph::{EvolutionOperator, Graph, Params};

pub fn components<F>(graph: &AsyncGraph, mut on_component: F)
where
    F: FnMut(StateSet) -> (),
{
    let num_states = graph.states().count();
    let mut sinks = StateSet::new(num_states);
    let fwd = graph.fwd();
    let bwd = graph.bwd();
    for s in graph.states() {
        let has_next = fwd
            .step(s)
            .fold(graph.empty_params(), |a, (_, b)| a.union(&b));
        let is_sink = graph.unit_params().minus(&has_next);
        if !is_sink.is_empty() {
            sinks.put(s, is_sink);
        }
    }
    let can_reach_sink = reach(&bwd, &sinks);
    on_component(sinks); // notify about the sinks we have found
    let initial = StateSet::new_with_fun(num_states, |i| {
        if let Some(sink) = can_reach_sink.get(i) {
            Some(graph.unit_params().minus(sink))
        } else {
            Some(graph.unit_params().clone())
        }
    });

    if initial.iter().next() == None {
        return;
    }

    let mut queue: Vec<StateSet> = Vec::new();
    queue.push(initial);

    while let Some(universe) = queue.pop() {
        println!("Universe state count: {} Remaining work queue: {}", universe.iter().count(), queue.len());
        let pivots = find_pivots(&universe);
        println!("Pivots state count: {}", pivots.iter().count());
        let forward = guarded_reach(&fwd, &pivots, &universe);
        let component_with_pivots = guarded_reach(&bwd, &pivots, &forward);
        let reachable_terminals = forward.minus(&component_with_pivots);

        let leaves_current = reachable_terminals
            .fold_union()
            .unwrap_or(graph.empty_params());
        let is_terminal = graph.unit_params().minus(&leaves_current);

        if !is_terminal.is_empty() {
            let terminal = StateSet::new_with_fun(num_states, |s| {
                component_with_pivots
                    .get(s)
                    .map(|p| p.intersect(&is_terminal))
            });
            on_component(terminal);
        }

        let basins_of_reachable_terminals = guarded_reach(&bwd, &forward, &universe);
        let empty = graph.empty_params();
        let unreachable_terminals = StateSet::new_with_fun(num_states, |s| {
            let in_basin = basins_of_reachable_terminals.get(s).unwrap_or(&empty);
            universe.get(s).map(|p| p.minus(in_basin))
        });

        if !leaves_current.is_empty() {
            queue.push(reachable_terminals);
        }
        if unreachable_terminals.iter().next() != None {
            queue.push(unreachable_terminals);
        }
    }
}

pub fn find_pivots(universe: &StateSet) -> StateSet {
    let mut result = StateSet::new(universe.capacity());
    let mut remaining = universe.fold_union().unwrap();
    for (s, p) in universe.iter() {
        let gain = remaining.intersect(p);
        if !gain.is_empty() {
            remaining = remaining.minus(&gain);
            result.put(s, gain);
            if remaining.is_empty() {
                return result;
            }
        }
    }
    unreachable!("Pivots can't be created.");
}
