use crate::scc::{StateSet, ProgressTracker};
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, InvertibleEvolutionOperator, Params};
use biodivine_lib_std::IdState;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

fn all_possible_successors<F>(fwd: &F, set: &HashSet<IdState>) -> HashSet<IdState> where
    F: EvolutionOperator<State = IdState, Params = BddParams> + Send + Sync
{
    return set.par_iter()
        .flat_map(|s| fwd.step(*s).map(|(t, _)| t).collect::<Vec<_>>())
        .collect();
}

trait FoldUnion {
    fn fold_union(self) -> Option<BddParams>;
}

impl<I> FoldUnion for I where I: Iterator<Item=Option<BddParams>> {
    fn fold_union(self) -> Option<BddParams> {
        return self.fold(None, |a, b| match (a, b) {
            (Some(a), Some(b)) => Some(a.union(&b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        });
    }
}

pub fn next_step<F, B>(fwd: &F, initial: &StateSet) -> StateSet where
    F: InvertibleEvolutionOperator<State = IdState, Params = BddParams, InvertedOperator = B> + Send + Sync,
    B: EvolutionOperator<State = IdState, Params = BddParams> + Send + Sync
{
    let bwd = fwd.invert();
    let items: Vec<(IdState, &BddParams)> = initial.iter().collect();
    let initial_states: HashSet<IdState> = items.iter().map(|(s, _)| *s).collect();
    let recompute: HashSet<IdState> = all_possible_successors(fwd, &initial_states);

    let result_items: Vec<(IdState, BddParams)> = recompute
        .par_iter()
        .filter_map(|t| {
            let add_to_t: Option<BddParams> = bwd
                .step(*t)
                .map(|(s, edge)| {
                    if let Some(initial_s) = initial.get(s) {
                        let s_to_t = initial_s.intersect(&edge);
                        if !s_to_t.is_empty() {
                            Some(s_to_t)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .fold_union();
            add_to_t.map(|p| (*t, p))
        })
        .collect();

    let mut result = StateSet::new(initial.capacity());
    for (s, p) in result_items {
        result.put(s, p);
    }

    return result;
}

pub fn guarded_reach<F, B>(fwd: &F, initial: &StateSet, guard: &StateSet, cancelled: &AtomicBool, progress: &ProgressTracker) -> StateSet
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

        if cancelled.load(Ordering::SeqCst) {
            return result_set;  // result is incorrect, but we are cancelled so we don't care...
        }

        progress.update_last_wave(changed.len());
        println!("Wave size: {}", changed.len());

        // All successors of changed states
        let recompute: HashSet<IdState> = all_possible_successors(fwd, &changed);

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
                                        // empty sets are pointless
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
                        .fold_union();
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

    progress.update_last_wave(0);

    return result_set;
}
