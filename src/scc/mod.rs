use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::IdState;
use std::collections::HashMap;

pub mod algo_components;
mod algo_par_reach;
mod algo_reach;
mod impl_class;
mod impl_classifier;
mod impl_state_set;
mod impl_state_set_iterator;

#[derive(Clone, Debug)]
pub struct StateSet(Vec<Option<BddParams>>);

pub struct StateSetIterator<'a> {
    set: &'a StateSet,
    next: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Behaviour {
    Stability,
    Oscillation,
    Disorder,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Class(Vec<Behaviour>);

pub struct Classifier<'a> {
    graph: &'a AsyncGraph,
    classes: HashMap<Class, BddParams>,
}

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
        // nothing found, restart once
        self.1 = 0;
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
        // nothing found, so there is nothing in there
        return None;
    }
}
