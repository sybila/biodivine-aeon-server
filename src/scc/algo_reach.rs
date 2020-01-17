use super::StateSet;
use crate::scc::CircularQueue;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Params};
use biodivine_lib_std::IdState;
use std::collections::HashSet;

pub fn guarded_reach<G>(fwd: &G, initial: &StateSet, guard: &StateSet) -> StateSet
where
    G: EvolutionOperator<State = IdState, Params = BddParams>,
{
    let mut result = initial.clone();
    let mut queue = CircularQueue::new(initial.capacity());

    // add initial states
    result.iter().for_each(|(s, _)| {
        queue.insert(s);
    });

    let mut iter = 0;
    while let Some(s) = queue.dequeue() {
        iter += 1;
        for (t, edge) in fwd.step(s) {
            if let Some(current) = result.get(s) {
                if let Some(guard) = guard.get(t) {
                    let to_add = current.intersect(&edge).intersect(guard);
                    if !to_add.is_empty() && result.union_key(t, &to_add) {
                        queue.insert(t);
                    }
                }
            }
        }
    }

    println!("Iters: {}", iter);

    return result;
}

pub fn reach<G>(fwd: &G, initial: &StateSet) -> StateSet
where
    G: EvolutionOperator<State = IdState, Params = BddParams>,
{
    let mut result = initial.clone();
    let mut queue = CircularQueue::new(initial.capacity());

    result.iter().for_each(|(s, _)| {
        queue.insert(s);
    });

    let mut iter = 0;
    while let Some(s) = queue.dequeue() {
        iter += 1;
        for (t, edge) in fwd.step(s) {
            if let Some(current) = result.get(s) {
                let to_add = current.intersect(&edge);
                if !to_add.is_empty() && result.union_key(t, &to_add) {
                    queue.insert(t);
                }
            }
        }
    }
    println!("Iters: {}", iter);

    return result;
}
