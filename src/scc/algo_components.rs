use super::StateSet;
use crate::scc::algo_par_reach::guarded_reach;
use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::async_graph::{AsyncGraph, FwdIterator};
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Graph, Params};
use biodivine_lib_std::IdState;
use rayon::prelude::*;
use std::option::Option::Some;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn components<F>(
    graph: &AsyncGraph,
    progress: &ProgressTracker,
    cancelled: &AtomicBool,
    on_component: F,
) where
    F: Fn(StateSet) -> () + Send + Sync,
{
    crossbeam::thread::scope(|scope| {
        let num_states = graph.states().count();
        let fwd = graph.fwd();
        let bwd = graph.bwd();
        println!("Start detecting sinks");
        let initial = StateSet::new_with_fun(num_states, |_| Some(graph.unit_params().clone()));
        let sink_pairs: Vec<(IdState, BddParams)> = graph
            .states()
            .collect::<Vec<_>>()
            .par_iter()
            .filter_map(|s| {
                let has_next = fwd
                    .step(*s)
                    .fold(graph.empty_params(), |a, (_, b)| a.union(&b));
                let is_sink = graph.unit_params().minus(&has_next);
                if !is_sink.is_empty() {
                    let mut sink_set = StateSet::new(num_states);
                    sink_set.put(*s, is_sink.clone());
                    scope.spawn(|_| {
                        on_component(sink_set);
                    });
                    Some((*s, is_sink))
                } else {
                    None
                }
            })
            .collect();

        if cancelled.load(Ordering::SeqCst) {
            return ();
        }

        println!("Sinks detected");

        let mut sinks = StateSet::new(num_states);
        for (s, is_sink) in sink_pairs {
            sinks.put(s, is_sink);
        }
        let can_reach_sink =
            guarded_reach(&bwd, &sinks, &initial, &AtomicBool::new(false), &progress);
        // This is not correct - on_component is called with individual components - sinks are multiple different components.
        //on_component(sinks); // notify about the sinks we have found
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

        let mut queue: Vec<(StateSet, f64)> = Vec::new();
        queue.push(with_cardinality(initial));

        while let Some((universe, universe_cardinality)) = queue.pop() {
            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            println!(
                "Universe state count: {} Remaining work queue: {}",
                universe.iter().count(),
                queue.len()
            );
            let remaining: f64 = queue.iter().map(|(_, b)| *b).sum::<f64>() + universe_cardinality;
            progress.update_remaining(remaining);
            let pivots = find_pivots(graph, &universe);
            println!("Pivots state count: {}", pivots.iter().count());
            let forward = guarded_reach(&fwd, &pivots, &universe, cancelled, &progress);

            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            let component_with_pivots =
                guarded_reach(&bwd, &pivots, &forward, cancelled, &progress);

            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

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
                scope.spawn(|_| {
                    on_component(terminal);
                });
            }

            let basins_of_reachable_terminals =
                guarded_reach(&bwd, &forward, &universe, cancelled, &progress);

            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            let empty = graph.empty_params();
            let unreachable_terminals = StateSet::new_with_fun(num_states, |s| {
                let in_basin = basins_of_reachable_terminals.get(s).unwrap_or(&empty);
                universe.get(s).map(|p| p.minus(in_basin))
            });

            if !leaves_current.is_empty() {
                queue.push(with_cardinality(reachable_terminals));
            }
            if unreachable_terminals.iter().next() != None {
                queue.push(with_cardinality(unreachable_terminals));
            }
        }
    })
    .unwrap();
}

// Augment a state set with the cardinality of the set
fn with_cardinality(set: StateSet) -> (StateSet, f64) {
    let cardinality = set.iter().map(|(_, p)| p.cardinality()).sum();
    return (set, cardinality);
}

pub fn find_pivots_basic(universe: &StateSet) -> StateSet {
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

pub fn find_pivots(graph: &AsyncGraph, universe: &StateSet) -> StateSet {
    /*let mut result = StateSet::new(universe.capacity());
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
    unreachable!("Pivots can't be created.");*/
    let mut remaining = universe.fold_union().unwrap();
    let mut result = StateSet::new(universe.capacity());
    while !remaining.is_empty() {
        let pivot = find_dfs_pivot(graph, universe, &remaining);
        let params = universe.get(pivot).unwrap().intersect(&remaining);
        remaining = remaining.minus(&params);
        result.union_key(pivot, &params);
    }
    return result;
}

pub fn find_dfs_pivot(graph: &AsyncGraph, universe: &StateSet, remaining: &BddParams) -> IdState {
    let mut visited = StateSet::new(universe.capacity());
    let mut stack: Vec<(IdState, BddParams, FwdIterator)> = Vec::new();
    let init = universe
        .iter()
        .map(|(s, p)| (s, p.intersect(remaining)))
        .filter(|(_, p)| !p.is_empty())
        .next()
        .unwrap(); // something must be found, otherwise someone messed up really bad
    let fwd = graph.fwd();
    visited.put(init.0, init.1.clone());
    stack.push((init.0, init.1, fwd.step(init.0)));
    while let Some((s, p, it)) = stack.last_mut() {
        if let Some((successor, edge)) = it.next() {
            if let Some(universe_params) = universe.get(successor) {
                let successor_params = p.intersect(&edge).intersect(universe_params);
                let successor_visited = visited.get(successor);
                let successor_params = if let Some(visited) = successor_visited {
                    successor_params.minus(visited)
                } else {
                    successor_params
                };
                if !successor_params.is_empty() {
                    visited.put(successor, successor_params.clone());
                    stack.push((successor, successor_params, fwd.step(successor)));
                }
            }
        } else {
            // this would be the first popped state, which is exactly the one we want to return
            return *s;
        }
    }
    unreachable!("Something must be popped. We won't get here.");
}
