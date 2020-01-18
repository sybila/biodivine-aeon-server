use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::bdd_params::BddParams;
use std::collections::HashMap;
use std::vec::IntoIter;
use std::cmp::Ordering;

pub mod algo_components;
pub mod algo_par_reach;
pub mod algo_reach;
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

pub struct StateSetIntoIterator {
    set: IntoIter<Option<BddParams>>,
    next: usize
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Behaviour {
    Stability,
    Oscillation,
    Disorder,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq)]
pub struct Class(Vec<Behaviour>);

/// Classes actually have a special ordering - primarily, they are ordered by the
/// number of behaviours, secondarily they are ordered by the actual behaviours.
impl PartialOrd for Class {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return if self.0.len() != other.0.len() {
            self.0.len().partial_cmp(&other.0.len())
        } else {
            if self.0.len() == 0 {
                Some(Ordering::Equal)
            } else {
                self.0.partial_cmp(&other.0)
            }
        }
    }
}

pub struct Classifier<'a> {
    graph: &'a AsyncGraph,
    classes: HashMap<Class, BddParams>,
}
