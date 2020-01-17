use biodivine_lib_param_bn::bdd_params::BddParams;
use std::collections::HashMap;
use biodivine_lib_param_bn::async_graph::AsyncGraph;

pub mod algo_components;
mod algo_reach;
mod impl_state_set;
mod impl_state_set_iterator;
mod impl_classifier;
mod impl_class;

#[derive(Clone, Debug)]
pub struct StateSet(Vec<Option<BddParams>>);

pub struct StateSetIterator<'a> {
    set: &'a StateSet,
    next: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Behaviour {
    Stability, Oscillation, Disorder
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Class(Vec<Behaviour>);

pub struct Classifier<'a> {
    graph: &'a AsyncGraph,
    classes: HashMap<Class, BddParams>
}