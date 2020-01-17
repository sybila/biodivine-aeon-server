use super::StateSet;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Params};
use biodivine_lib_std::IdState;
use std::collections::HashSet;

pub fn guarded_reach<G>(fwd: &G, initial: &StateSet, guard: &StateSet) -> StateSet
where
    G: EvolutionOperator<State = IdState, Params = BddParams>,
{
    let mut result = initial.clone();
    let mut queue: HashSet<IdState> = HashSet::new();

    // add initial states
    result.iter().for_each(|(s, _)| {
        queue.insert(s);
    });

    while let Some(s) = queue.iter().next().map(|s| *s) {
        queue.remove(&s);
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

    return result;
}

pub fn reach<G>(fwd: &G, initial: &StateSet) -> StateSet
where
    G: EvolutionOperator<State = IdState, Params = BddParams>,
{
    let mut result = initial.clone();
    let mut queue: HashSet<IdState> = HashSet::new();

    result.iter().for_each(|(s, _)| {
        queue.insert(s);
    });

    while let Some(s) = queue.iter().next().map(|s| *s) {
        queue.remove(&s);
        for (t, edge) in fwd.step(s) {
            if let Some(current) = result.get(s) {
                let to_add = current.intersect(&edge);
                if !to_add.is_empty() && result.union_key(t, &to_add) {
                    queue.insert(t);
                }
            }
        }
    }

    return result;
}
