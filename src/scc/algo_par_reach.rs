use crate::scc::StateSet;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Graph, Params};
use biodivine_lib_std::IdState;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// Parallel reach procedure works in waves:
///  - Each wave has an input of `changed` states - these are the states where the value
///  was changed in the last wave (or initial states for the first wave).
///  - We flat-map each state in a wave to a set of its successor edges and for each edge,
///  we evaluate whether the edge can extend the parameter set in its target state.
///  - Finally, we fold-reduce these into a new state set of `changed` states.
///  - In the end, we can (sequentially) put these into the result state set and create
///  the set of `changed` states for the next wave.
pub fn guarded_reach<G>(fwd: &G, initial: &StateSet, guard: &StateSet) -> StateSet
    where G: EvolutionOperator<State=IdState, Params=BddParams> + Send + Sync
{
    let capacity = initial.capacity();
    let mut result_set = StateSet::new(capacity);
    let mut changed = HashSet::new();
    for (s, p) in initial.iter() {
        result_set.put(s, p.clone());
        changed.insert(s);
    }

    while !changed.is_empty() {
        println!("Wave size: {}", changed.len());
        let wave: StateSet = changed
            .par_iter()
            .flat_map(|s: &IdState| {
                let edges: Vec<_> = fwd.step(*s).map(|(t, p)| (*s, t, p)).collect();
                edges
            })
            .filter_map(|(s, t, p)| {
                let value_s = result_set.get(s);
                let value_t = result_set.get(t);
                let guard_t = guard.get(t);
                if let Some(guard_t) = guard_t {
                    if let Some(value_s) = value_s {
                        let add_to_t = value_s.intersect(&p).intersect(guard_t);
                        if !add_to_t.is_empty() {   // only add if the set is not empty
                            if let Some(value_t) = value_t {
                                if !(add_to_t.is_subset(value_t)) {
                                    // add_to_t actually contains new stuff!
                                    // send it out with the old stuff as well so that we can later just put it where it belongs without union
                                    Some((t, add_to_t.union(value_t)))
                                } else {
                                    None
                                }
                            } else {    // t has no value and we have a suggestion for a new one
                                Some((t, add_to_t))
                            }
                        } else {    // if add_to_t is empty, just skip
                            None
                        }
                    } else {    // s has no value - this should not happen, but its an easy None
                        None
                    }
                } else {    // guard is empty, we can't put anything into that state anyway...
                    None
                }
            })
            .fold(|| StateSet::new(capacity), |mut set, (t, add)| {
                set.union_key(t, &add); set
            })
            .reduce(|| StateSet::new(capacity), |mut a, b| {
                for (t, add) in b.iter() {
                    a.union_key(t, add);
                }
                a
            });

        changed.clear();
        for (s, p) in wave.iter() {
            // we don't have to union because we did that in the parallel iterator
            result_set.put(s, p.clone());
            changed.insert(s);
        }
    }

    return result_set;
}