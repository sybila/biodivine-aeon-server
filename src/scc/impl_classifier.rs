use super::{Behaviour, Class, Classifier, StateSet};
use crate::scc::algo_components::{find_pivots, find_pivots_basic};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Graph, Params};
use std::collections::HashMap;

impl<'a> Classifier<'a> {
    pub fn new(graph: &AsyncGraph) -> Classifier {
        let mut map: HashMap<Class, BddParams> = HashMap::new();
        map.insert(Class::new_empty(), graph.unit_params().clone());
        return Classifier {
            graph,
            classes: map,
        };
    }

    pub fn export_result(self) -> HashMap<Class, BddParams> {
        return self.classes;
    }

    pub fn add_component(&mut self, component: StateSet) {
        // first, remove all sink states
        let component_without_sinks = StateSet::new_with_fun(component.capacity(), |s| {
            if let Some(in_component) = component.get(s) {
                let has_next = self
                    .graph
                    .fwd()
                    .step(s)
                    .fold(self.graph.empty_params(), |a, (_, b)| a.union(&b));
                let is_sink = in_component.minus(&has_next);
                if !is_sink.is_empty() {
                    self.push(Behaviour::Stability, &is_sink);
                }
                Some(has_next.intersect(in_component))
            } else {
                None
            }
        });

        let not_sink_params = component_without_sinks.fold_union();
        if let Some(not_sink_params) = not_sink_params {
            let pivots = find_pivots_basic(&component_without_sinks);
            let mut oscillator =
                Oscillator::new_with_pivots(pivots.clone(), self.graph.empty_params());

            let mut disorder = self.graph.empty_params();
            let mut params_to_match = not_sink_params.clone();
            let mut current_level = pivots;

            while !params_to_match.is_empty() {
                let fwd = self.graph.fwd();
                let mut reachable = StateSet::new(component.capacity());
                for (s, current_s) in current_level.iter() {
                    for (t, edge) in fwd.step(s) {
                        let target = current_s.intersect(&edge).intersect(&params_to_match);
                        reachable.union_key(t, &target);
                    }
                }

                let (not_oscillating, continue_with) = oscillator.push_wave(&reachable);
                disorder = disorder.union(&not_oscillating);
                params_to_match = params_to_match.intersect(&continue_with);
                current_level = reachable;
            }

            let oscillates = not_sink_params.minus(&disorder);

            if !disorder.is_empty() {
                self.push(Behaviour::Disorder, &disorder);
            }

            if !oscillates.is_empty() {
                self.push(Behaviour::Oscillation, &oscillates);
            }
        }
    }

    fn push(&mut self, class: Behaviour, params: &BddParams) {
        let mut original_classes: Vec<Class> = self.classes.keys().map(|c| c.clone()).collect();
        original_classes.sort();
        original_classes.reverse(); // we need classes from largest to smallest

        for c in original_classes {
            let c_p = &self.classes[&c];
            let should_move_up = c_p.intersect(&params);
            if !should_move_up.is_empty() {
                let mut larger = c.clone();
                larger.extend(class);

                // remove moving params from c
                let new_c_p = c_p.minus(&should_move_up);
                if new_c_p.is_empty() {
                    self.classes.remove(&c);
                } else {
                    self.classes.insert(c, new_c_p);
                }

                // add moving params to larger_class
                if let Some(larger_p) = self.classes.get(&larger) {
                    let new_larger = larger_p.union(&should_move_up);
                    self.classes.insert(larger, new_larger);
                } else {
                    self.classes.insert(larger, should_move_up);
                }
            }
        }
    }

    pub fn print(&self) {
        for (c, p) in &self.classes {
            println!("Class {:?}, cardinality: {}", c, p.cardinality());
        }
    }
}

struct Oscillator(Vec<StateSet>, BddParams);

impl Oscillator {
    pub fn new_with_pivots(pivots: StateSet, empty: BddParams) -> Oscillator {
        return Oscillator(vec![pivots], empty);
    }

    pub fn push_wave(&mut self, wave: &StateSet) -> (BddParams, BddParams) {
        let wave_params = wave.fold_union().unwrap();
        /*
         * First, compute sets of parameters for which the wave intersects each class.
         *
         * If some parameters intersect two classes, these do not oscillate. If some parameters intersect no
         * class, these need to be pushed to a new class.
         */
        let mut already_found = self.1.clone();
        let mut not_oscillating = self.1.clone();
        let mut new_class = wave_params;
        let mut intersections: Vec<BddParams> = Vec::new();
        for class in &self.0 {
            let mut class_wave_intersection = self.1.clone();
            for (s, class_p) in class.iter() {
                if let Some(wave_p) = wave.get(s) {
                    class_wave_intersection =
                        class_wave_intersection.union(&class_p.intersect(wave_p));
                }
            }
            let no_oscillation = already_found.intersect(&class_wave_intersection); // parameters which already have intersection
            not_oscillating = not_oscillating.union(&no_oscillation);
            already_found = already_found.union(&class_wave_intersection);
            new_class = new_class.minus(&class_wave_intersection); // remove discovered parameters
            intersections.push(class_wave_intersection);
        }

        if !new_class.is_empty() {
            let class = StateSet::new_with_fun(wave.capacity(), |s| {
                wave.get(s).map(|p| p.intersect(&new_class))
            });
            self.0.push(class);
        }

        let mut continue_params = new_class;
        // now union wave based on intersections
        for c_i in 0..intersections.len() {
            let class_intersection = &intersections[c_i];
            for (s, wave_p) in wave.iter() {
                let state_params = wave_p.minus(&not_oscillating).intersect(class_intersection);
                if !state_params.is_empty() {
                    if self.0[c_i].union_key(s, &state_params) {
                        continue_params = continue_params.union(&state_params)
                    }
                }
            }
        }

        return (not_oscillating, continue_params);
    }
}
