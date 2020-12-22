use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, GraphColors};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Mutex;
use std::vec::IntoIter;

/// **(internal)** Utility methods for the behaviour `Class`.
mod _impl_class;
/// **(internal)** Implementation of `Behaviour` classification in `Classifier`.
mod _impl_classifier;
pub mod algo_effectively_constant;
pub mod algo_symbolic_components;
mod impl_progress_tracker;
mod impl_state_set_iterator;

#[derive(Clone, Debug)]
pub struct StateSet(Vec<Option<BddParams>>);

pub struct StateSetIterator<'a> {
    set: &'a StateSet,
    next: usize,
}

pub struct StateSetIntoIterator {
    set: IntoIter<Option<BddParams>>,
    next: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Behaviour {
    Stability,
    Oscillation,
    Disorder,
}

// TODO: This is super inefficient - we have three behaviours, just make this a (usize, usize, usize).
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
        };
    }
}

pub struct Classifier {
    //graph: &'a AsyncGraph,
    classes: Mutex<HashMap<Class, GraphColors>>,
    attractors: Mutex<Vec<(GraphColoredVertices, HashMap<Behaviour, GraphColors>)>>,
}

pub struct ProgressTracker {
    total: f64,
    remaining: Mutex<f64>,
    last_wave: Mutex<f64>,
}
