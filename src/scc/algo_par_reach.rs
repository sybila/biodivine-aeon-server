use crate::scc::StateSet;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Graph, Params};
use biodivine_lib_std::IdState;
use rayon::prelude::*;
use std::collections::HashMap;

pub fn guarded_reach_fwd(graph: &AsyncGraph, initial: &StateSet, guard: &StateSet) -> StateSet {
    println!("Start parallel reachability.");
    let mut result = Vec::new();
    for state in graph.states() {
        if let Some(value) = initial.get(state) {
            result.push(value.clone());
        } else {
            result.push(graph.empty_params());
        }
    }

    let mut step = parallel_step_fwd(graph, &result);
    while !step.is_empty() {
        println!("Reachability wave: {}", step.len());
        step.into_iter().for_each(|(s, p)| {
            let index: usize = s.into();
            result[index] = p
        });
        step = parallel_step_fwd(graph, &result);
    }

    let mut r = StateSet::new(graph.num_states());
    for s in 0..graph.num_states() {
        let p = &result[s];
        if !p.is_empty() {
            r.put(IdState::from(s), p.clone());
        }
    }
    return r;
}

pub fn guarded_reach_bwd(graph: &AsyncGraph, initial: &StateSet, guard: &StateSet) -> StateSet {
    println!("Start parallel reachability.");
    let mut result = Vec::new();
    for state in graph.states() {
        if let Some(value) = initial.get(state) {
            result.push(value.clone());
        } else {
            result.push(graph.empty_params());
        }
    }

    let mut step = parallel_step_bwd(graph, &result);
    while !step.is_empty() {
        println!("Reachability wave: {}", step.len());
        step.into_iter().for_each(|(s, p)| {
            let index: usize = s.into();
            result[index] = p
        });
        step = parallel_step_bwd(graph, &result);
    }

    let mut r = StateSet::new(graph.num_states());
    for s in 0..graph.num_states() {
        let p = &result[s];
        if !p.is_empty() {
            r.put(IdState::from(s), p.clone());
        }
    }
    return r;
}

/// Given the current values for each state, compute a hash map of states where new value can be
/// added to the current vector.
fn parallel_step_fwd(graph: &AsyncGraph, current: &Vec<BddParams>) -> HashMap<IdState, BddParams> {
    let states: Vec<IdState> = graph.states().collect();
    let bwd = graph.bwd();
    let updated: HashMap<IdState, BddParams> = states
        .into_par_iter()
        .filter_map(|s: IdState| {
            let index: usize = s.into();
            let old = &current[index];
            let mut new = graph.empty_params();
            for (predecessor, edge) in bwd.step(s) {
                let p_index: usize = predecessor.into();
                let p_params = &current[p_index];
                new = new.union(&p_params.intersect(&edge));
            }
            if new.is_subset(old) {
                None
            } else {
                Some((s, new.union(old)))
            }
        })
        .collect();
    return updated;
}

/// Given the current values for each state, compute a hash map of states where new value can be
/// added to the current vector.
fn parallel_step_bwd(graph: &AsyncGraph, current: &Vec<BddParams>) -> HashMap<IdState, BddParams> {
    let states: Vec<IdState> = graph.states().collect();
    let bwd = graph.fwd();
    let updated: HashMap<IdState, BddParams> = states
        .into_par_iter()
        .filter_map(|s: IdState| {
            let index: usize = s.into();
            let old = &current[index];
            let mut new = graph.empty_params();
            for (predecessor, edge) in bwd.step(s) {
                let p_index: usize = predecessor.into();
                let p_params = &current[p_index];
                new = new.union(&p_params.intersect(&edge));
            }
            if new.is_subset(old) {
                None
            } else {
                Some((s, new.union(old)))
            }
        })
        .collect();
    return updated;
}
