use super::{Behaviour, Class, Classifier, StateSet};
use biodivine_lib_param_bn::async_graph::{AsyncGraph, DefaultEdgeParams};
use biodivine_lib_param_bn::bdd_params::BddParams;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(feature = "extended_oscillation")]
use crate::scc::algo_components::find_pivots_basic;
#[cfg(feature = "extended_oscillation")]
use crate::scc::algo_par_reach::next_step;
use biodivine_lib_param_bn::biodivine_std::structs::IdState;
use biodivine_lib_param_bn::biodivine_std::traits::{EvolutionOperator, Graph, Set};

impl Classifier {
    pub fn new(graph: &AsyncGraph<DefaultEdgeParams>) -> Classifier {
        let mut map: HashMap<Class, BddParams> = HashMap::new();
        map.insert(Class::new_empty(), graph.unit_params().clone());
        return Classifier {
            classes: Mutex::new(map),
        };
    }

    // Try to fetch the current number of discovered classes in a non-blocking manner
    pub fn try_get_num_classes(&self) -> Option<usize> {
        return match self.classes.try_lock() {
            Ok(data) => Some((*data).len()),
            _ => None,
        };
    }

    // Try to obtain a copy of data in a non-blocking manner (useful if we want to check
    // results but the computation is still running).
    pub fn try_export_result(&self) -> Option<HashMap<Class, BddParams>> {
        return match self.classes.try_lock() {
            Ok(data) => Some((*data).clone()),
            _ => None,
        };
    }

    pub fn try_get_params(&self, class: &Class) -> Option<Option<BddParams>> {
        return match self.classes.try_lock() {
            Ok(data) => Some((*data).get(class).map(|p| p.clone())),
            _ => None,
        };
    }

    pub fn get_params(&self, class: &Class) -> Option<BddParams> {
        let data = self.classes.lock().unwrap();
        return (*data).get(class).map(|p| p.clone());
    }

    pub fn export_result(&self) -> HashMap<Class, BddParams> {
        let data = self.classes.lock().unwrap();
        return (*data).clone();
    }

    pub fn add_component(&self, component: StateSet, graph: &AsyncGraph<DefaultEdgeParams>) {
        let without_sinks = self.filter_sinks(component, graph);
        if let Some(not_sink_params) = without_sinks.fold_union() {
            let fwd = graph.fwd();
            let mut not_cycle = graph.empty_params().clone();
            for (s, p) in without_sinks.iter() {
                let mut to_be_seen = p.clone(); // sinks are removed, so there must be an edge for every parameter
                let mut seen_more_than_once = graph.empty_params().clone();
                for (successor, edge_params) in fwd.step(s) {
                    // Parameters for which this edge (s -> successor) is in the attractor.
                    let successor_params = p.intersect(&edge_params).intersect(
                        without_sinks
                            .get(successor)
                            .unwrap_or(&graph.empty_params()),
                    );

                    // Parameters which were already seen for some previous edge.
                    let already_seen = successor_params.minus(&to_be_seen);
                    seen_more_than_once = seen_more_than_once.union(&already_seen);

                    // Mark all of this as seen.
                    to_be_seen = to_be_seen.minus(&successor_params);
                }
                // Everything that was seen more than once is not in a cycle
                not_cycle = not_cycle.union(&seen_more_than_once);
            }
            let cycle = not_sink_params.minus(&not_cycle);
            if !not_cycle.is_empty() {
                self.push(Behaviour::Disorder, not_cycle);
            }
            if !cycle.is_empty() {
                self.push(Behaviour::Oscillation, cycle);
            }
        }
    }

    fn push(&self, behaviour: Behaviour, params: BddParams) {
        let mut classes = self.classes.lock().unwrap();
        let mut original_classes: Vec<Class> = (*classes).keys().map(|c| c.clone()).collect();
        original_classes.sort();
        original_classes.reverse(); // we need classes from largest to smallest

        for class in original_classes {
            let class_params = &(*classes)[&class];
            let should_move_up = class_params.intersect(&params);
            if !should_move_up.is_empty() {
                let extended_class = class.clone_extended(behaviour);

                // remove moving params from class
                let new_c_p = class_params.minus(&should_move_up);
                if new_c_p.is_empty() {
                    (*classes).remove(&class);
                } else {
                    (*classes).insert(class, new_c_p);
                }

                // add moving params to larger_class
                if let Some(extended_class_params) = (*classes).get(&extended_class) {
                    let new_extended_params = extended_class_params.union(&should_move_up);
                    (*classes).insert(extended_class, new_extended_params);
                } else {
                    (*classes).insert(extended_class, should_move_up);
                }
            }
        }
    }

    pub fn print(&self) {
        let classes = self.classes.lock().unwrap();
        for (c, p) in &(*classes) {
            println!("Class {:?}, cardinality: {}", c, p.cardinality());
        }
    }

    /// Remove all sink states from the given component (and push them into the classifier).
    fn filter_sinks(&self, component: StateSet, graph: &AsyncGraph<DefaultEdgeParams>) -> StateSet {
        let fwd = graph.fwd();
        let mut result = component.clone();
        let data: Vec<(IdState, BddParams)> = component.into_iter().collect();
        let processed: Vec<(IdState, BddParams, BddParams)> = data
            .par_iter()
            .filter_map(|(s, p): &(IdState, BddParams)| {
                let has_successor = fwd
                    .step(*s)
                    .fold(graph.empty_params().clone(), |a, (_, b)| a.union(&b));
                let is_sink = p.minus(&has_successor);
                if is_sink.is_empty() {
                    None
                } else {
                    let remaining = p.intersect(&has_successor);
                    Some((*s, is_sink, remaining))
                }
            })
            .collect();

        for (state, is_sink, remaining) in processed {
            self.push(Behaviour::Stability, is_sink);
            if remaining.is_empty() {
                result.clear_key(state);
            } else {
                result.put(state, remaining);
            }
        }

        return result;
    }
}
