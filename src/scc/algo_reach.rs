//! Sequential implementation of basic algorithms for computing states of sets that you can
//! reach from some initial states.

use super::StateSet;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_param_bn::biodivine_std::structs::IdState;
use biodivine_lib_param_bn::biodivine_std::traits::{EvolutionOperator, Set};

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

    while let Some(s) = queue.dequeue() {
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
    let mut queue = CircularQueue::new(initial.capacity());

    result.iter().for_each(|(s, _)| {
        queue.insert(s);
    });

    while let Some(s) = queue.dequeue() {
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

/// A small utility queue of a fixed size that behaves as a set (no duplicates).
/// Unfortunately, dequeue can take linear time, however, this should be still
/// fairly reasonable for systems where parameters are the real problem.
///
/// Interestingly, it gives a very nice performance when exploring the graph
/// (at least compared to pure bfs, random, or, god forbid, pure dfs.
pub struct CircularQueue(Vec<bool>, usize);

impl CircularQueue {
    pub fn new(capacity: usize) -> CircularQueue {
        return CircularQueue(vec![false; capacity], 0);
    }
    pub fn insert(&mut self, state: IdState) {
        let index: usize = state.into();
        self.0[index] = true;
    }

    pub fn dequeue(&mut self) -> Option<IdState> {
        // try to finish current run
        return self.try_find().or_else(|| {
            self.1 = 0;
            self.try_find()
        });
    }

    fn try_find(&mut self) -> Option<IdState> {
        while self.1 < self.0.len() {
            if self.0[self.1] {
                let result = IdState::from(self.1);
                self.0[self.1] = false;
                self.1 += 1;
                return Some(result);
            } else {
                self.1 += 1;
            }
        }
        return None;
    }
}
