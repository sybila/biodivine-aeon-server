use crate::scc::StateSet;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, InvertibleEvolutionOperator, Params};
use biodivine_lib_std::IdState;
use rayon::prelude::*;
use std::collections::HashSet;

pub fn guarded_reach<F, B>(fwd: &F, initial: &StateSet, guard: &StateSet) -> StateSet
where
    F: InvertibleEvolutionOperator<State = IdState, Params = BddParams, InvertedOperator = B>
        + Send
        + Sync,
    B: EvolutionOperator<State = IdState, Params = BddParams> + Send + Sync,
{
    let bwd = fwd.invert();
    let capacity = initial.capacity();
    let mut result_set = StateSet::new(capacity);
    let mut changed = HashSet::new();
    for (s, p) in initial.iter() {
        result_set.put(s, p.clone());
        changed.insert(s);
    }

    while !changed.is_empty() {
        println!("Wave size: {}", changed.len());

        // All successors of changed states
        let recompute: HashSet<IdState> = changed
            .par_iter()
            .flat_map(|s| fwd.step(*s).map(|(t, _)| t).collect::<Vec<_>>())
            .collect();

        let update: Vec<(IdState, BddParams)> = recompute
            .par_iter()
            .filter_map(|t| {
                let guard_t = guard.get(*t);
                if let Some(guard_t) = guard_t {
                    let add_to_t: Option<BddParams> = bwd
                        .step(*t)
                        .map(|(s, edge)| {
                            if changed.contains(&s) {
                                let current_s = result_set.get(s);
                                if let Some(current_s) = current_s {
                                    let s_adds_to_t = current_s.intersect(&edge).intersect(guard_t);
                                    if !s_adds_to_t.is_empty() {
                                        Some(s_adds_to_t)
                                    } else {
                                        // empy sets are pointless
                                        None
                                    }
                                } else {
                                    // if current_s is empty, s won't contribute anything
                                    None
                                }
                            } else {
                                // if s was not changed, it contributes nothing
                                None
                            }
                        })
                        .fold(None, |a, b| match (a, b) {
                            (Some(a), Some(b)) => Some(a.union(&b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        });
                    let current_t = result_set.get(*t);
                    let add_to_t = if let Some(current_t) = current_t {
                        // if there already is something in t, we have to check if we are adding something new
                        add_to_t
                            .filter(|p| !p.is_subset(current_t))
                            .map(|p| p.union(current_t))
                    } else {
                        add_to_t
                    };
                    add_to_t.map(|p| (*t, p))
                } else {
                    // guard is empty... no need to compute anything
                    None
                }
            })
            .collect();

        changed.clear();
        for (s, p) in update {
            changed.insert(s);
            result_set.put(s, p);
        }
    }

    return result_set;
}
