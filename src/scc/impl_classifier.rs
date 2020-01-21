use super::{Behaviour, Class, Classifier, StateSet};
use crate::scc::algo_components::find_pivots_basic;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::{EvolutionOperator, Graph, Params};
use std::collections::HashMap;
use rayon::prelude::*;
use biodivine_lib_std::IdState;
use std::sync::Mutex;

impl<'a> Classifier<'a> {
    pub fn new(graph: &AsyncGraph) -> Classifier {
        let mut map: HashMap<Class, BddParams> = HashMap::new();
        map.insert(Class::new_empty(), graph.unit_params().clone());
        return Classifier {
            graph,
            classes: Mutex::new(map),
        };
    }

    pub fn export_result(self) -> HashMap<Class, BddParams> {
        let data = self.classes.lock().unwrap();
        return (*data).clone();
    }

    pub fn add_component(&self, component: StateSet) {
        // first, remove all sink states
        let capacity = component.capacity();
        let without_sinks = self.filter_sinks(component);
        //let (real_oscillation, real_disorder) = self.decide_oscillation_vs_disorder(without_sinks.clone());

        let not_sink_params = without_sinks.fold_union();
        if let Some(not_sink_params) = not_sink_params {
            let pivots = find_pivots_basic(&without_sinks);
            let mut oscillator =
                Oscillator::new_with_pivots(pivots.clone(), self.graph.empty_params());

            let mut disorder = self.graph.empty_params();
            let mut params_to_match = not_sink_params.clone();
            let mut current_level = pivots;

            while !params_to_match.is_empty() {
                //println!("Simulation step size: {:?} cardinality: {}, history: {}", current_level.iter().count(), current_level.fold_union().unwrap().cardinality(), oscillator.0.len());
                let fwd = self.graph.fwd();
                let mut reachable = StateSet::new(capacity);
                //println!("Current: {:?}", current_level.cardinalities());
                for (s, current_s) in current_level.iter() {
                    for (t, edge) in fwd.step(s) {
                        let target = current_s.intersect(&edge).intersect(&params_to_match);
                        if !target.is_empty() {
                            reachable.union_key(t, &target);
                        }
                    }
                }
                //println!("Reachable: {:?}", reachable.cardinalities());

                let (not_oscillating, continue_with) = oscillator.push_wave(&reachable);
                disorder = disorder.union(&not_oscillating);
                params_to_match = params_to_match.intersect(&continue_with);
                current_level = reachable;
            }

            let oscillates = not_sink_params.minus(&disorder);

            if !disorder.is_empty() {
                self.push(Behaviour::Disorder, disorder);
            }

            if !oscillates.is_empty() {
                self.push(Behaviour::Oscillation, oscillates);
            }

            /*if !real_disorder.is_subset(&disorder) {
                panic!("Found disorder which old marked as oscialltion.")
            }

            if !real_oscillation.is_subset(&oscillates) {
                let new_oscillation = real_oscillation.minus(&oscillates);
                let witness = self.graph.make_witness(&new_oscillation);
                println!("{}", witness);
                panic!("Found oscillation which old marked as disorder");
            }*/
        }
    }

    fn push(&self, behaviour: Behaviour, params: BddParams) {
        let mut classes = self.classes.lock().unwrap();
        let mut original_classes: Vec<Class> = (*classes).keys().map(|c| c.clone()).collect();
        original_classes.sort();
        original_classes.reverse(); // we need classes from largest to smallest

        for class in original_classes {
            let class_params= &(*classes)[&class];
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
    fn filter_sinks(&self, component: StateSet) -> StateSet {
        let fwd = self.graph.fwd();
        let mut result = component.clone();
        let data: Vec<(IdState, BddParams)> = component.into_iter().collect();
        let processed: Vec<(IdState, BddParams, BddParams)> = data.par_iter()
            .filter_map(|(s, p): &(IdState, BddParams)| {
                let has_successor = fwd
                    .step(*s)
                    .fold(self.graph.empty_params(), |a, (_, b)| {
                        a.union(&b)
                    });
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

    /*
    /// Split the parameters in the component between oscillating and disordered
    /// (and push them into the classifier).
    ///
    /// The algorithm works as follows: We are going to pick a pivot for each parametrisation
    /// and start a symbolic "simulation" step by step from this pivot. Each step of the
    /// simulation is pushed into a history vector (as long as its not repeating).
    ///
    /// If the component is oscillating, the component can be exactly partitioned into finitely
    /// many simulation steps such that there are always edges only from one step to the next
    /// (and from last to first). If the component is disordered, this is going to break
    /// at some point and we will have a simulation step that intersects more than one
    /// history step.
    ///
    fn decide_oscillation_vs_disorder(&mut self, component: StateSet) -> (BddParams, BddParams) {
        if let Some(to_decide) = component.fold_union() {
            let fwd = self.graph.fwd();
            let mut history: Vec<StateSet> = Vec::new();
            let mut simulation_step = find_pivots_basic(&component);
            history.push(simulation_step.clone());

            // Initially, we assume everything oscillates and we iteratively move
            // disordered parameters into the correct set.
            let mut oscillation = to_decide.clone();
            let mut disorder = self.graph.empty_params();

            while simulation_step.iter().next() != None {
                println!("Remaining: {} History: {}, size: {}", simulation_step.fold_union().unwrap().cardinality(), history.len(), simulation_step.iter().count());
                println!("Before: {:?}", simulation_step.cardinalities());
                simulation_step = next_step(&fwd, &simulation_step);
                println!("After: {:?}", simulation_step.cardinalities());

                // Here, we keep the parameters that were already found in some history
                // step, so that we can detect if they appeared again.
                let mut found_in_history = self.graph.empty_params();
                // Here, we will keep the part of the simulation step that actually needs to
                // have a new history step created (some things can remain in simulation
                // but be assigned to existing steps if they already have some parameters there).
                let mut new_history_step = simulation_step.clone();
                for history_step in history.iter_mut() {
                    // Set of parameters in the simulation step that intersect with this history
                    // step. The intersection must occur in the same states!
                    // This is basically optimized version of:
                    // history_step.intersect(simulation_step).fold_union()
                    let history_step_intersection = history_step.iter()
                        .fold(self.graph.empty_params(), |result, (s, params_in_history)| {
                            if let Some(params_in_simulation) = simulation_step.get(s) {
                                result.union(&params_in_simulation.intersect(params_in_history))
                            } else {
                                result
                            }
                        });

                    if history_step_intersection.is_empty() {
                        continue;
                    }

                    // Detect inconsistencies in history, i.e. disorder.
                    let duplicate_history = found_in_history.intersect(&history_step_intersection);
                    if !duplicate_history.is_empty() {
                        //println!("Found disorder: {}", duplicate_history.cardinality());
                        // These are the disordered parameters that cannot oscillate.
                        oscillation = oscillation.minus(&duplicate_history);
                        disorder = disorder.union(&duplicate_history);
                    }
                    found_in_history = found_in_history.union(&history_step_intersection);

                    // Now remove the things that we already have in history from our simulation.
                    // These we do not have to concern ourselves with any more.
                    simulation_step.minus_in_place(&history_step);

                    // And add things that we still have in our simulation and we just classified
                    // they belong into this history step:
                    for (s, in_simulation) in simulation_step.iter() {
                        let should_be_in_this_step = in_simulation.intersect(&history_step_intersection);
                        if !should_be_in_this_step.is_empty() {
                            history_step.union_key(s, &should_be_in_this_step);
                        }
                    }

                    // Subtract this AFTER so that we ensure even items newly added to the history
                    // will be removed (since they don't need a new step, they already belong here).
                    new_history_step.minus_in_place(&history_step);
                }

                println!("After sorting: {:?}", simulation_step.cardinalities());

                if new_history_step.iter().next() != None {
                    history.push(new_history_step)
                }
            }

            println!("{:?}", history.iter().map(|step| {
                step.cardinalities()
            }).collect::<Vec<_>>());

            // At this point the oscillation and disorder sets should be correctly partitioned
            if !oscillation.is_empty() {
                self.push(Behaviour::Oscillation, oscillation.clone());
            }
            if !disorder.is_empty() {
                self.push(Behaviour::Disorder, disorder.clone());
            }

            return (oscillation, disorder)
        }   // component is empty, nothing to do here...
         else {
             return (self.graph.empty_params(), self.graph.empty_params())
         }
    }*/

}

/// Oscillator partitions the
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
