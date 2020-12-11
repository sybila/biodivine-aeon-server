use super::{Behaviour, Class, Classifier};
use biodivine_lib_param_bn::symbolic_async_graph::{
    GraphColoredVertices, GraphColors, GraphVertices, SymbolicAsyncGraph,
};
use biodivine_lib_std::param_graph::Params;
use std::collections::HashMap;
use std::sync::Mutex;

impl Classifier {
    pub fn new(graph: &SymbolicAsyncGraph) -> Classifier {
        let mut map: HashMap<Class, GraphColors> = HashMap::new();
        map.insert(Class::new_empty(), graph.unit_colors().clone());
        return Classifier {
            classes: Mutex::new(map),
            attractors: Mutex::new(Vec::new()),
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
    pub fn try_export_result(&self) -> Option<HashMap<Class, GraphColors>> {
        return match self.classes.try_lock() {
            Ok(data) => Some((*data).clone()),
            _ => None,
        };
    }

    pub fn try_get_params(&self, class: &Class) -> Option<Option<GraphColors>> {
        return match self.classes.try_lock() {
            Ok(data) => Some((*data).get(class).map(|p| p.clone())),
            _ => None,
        };
    }

    pub fn get_params(&self, class: &Class) -> Option<GraphColors> {
        let data = self.classes.lock().unwrap();
        return (*data).get(class).map(|p| p.clone());
    }

    pub fn export_result(&self) -> HashMap<Class, GraphColors> {
        let data = self.classes.lock().unwrap();
        return (*data).clone();
    }

    /// Static function to classify just one component and immediately obtain results.
    pub fn classify_component(
        component: &GraphColoredVertices,
        graph: &SymbolicAsyncGraph,
    ) -> HashMap<Behaviour, GraphColors> {
        let classifier = Classifier::new(graph);
        classifier.add_component(component.clone(), graph);
        let mut result: HashMap<Behaviour, GraphColors> = HashMap::new();
        for (class, colors) in classifier.export_result() {
            if class.0.is_empty() {
                continue; // This is an empty class - those colors were not in the attractor.
            } else if class.0.len() > 1 {
                unreachable!("Multiple behaviours in one component.");
            } else {
                result.insert(class.0[0], colors);
            }
        }
        return result;
    }

    /// Find attractor of the given witness colour. The argument set must be a singleton.
    pub fn attractors(
        &self,
        witness_colour: &GraphColors,
        graph: &SymbolicAsyncGraph,
    ) -> Vec<(GraphVertices, Behaviour)> {
        if witness_colour.cardinality() != 1.0 {
            eprintln!("WARNING: Computing attractor witnesses for non-singleton set. (This may be just a floating point error in large models).");
        }
        let mut result = Vec::new();
        let attractors = self.attractors.lock().unwrap();
        for (attractor, behaviour) in attractors.iter() {
            let attractor_states = attractor.intersect_colors(witness_colour);
            if attractor_states.is_empty() {
                continue;
            }
            let attractor_states = attractor_states.state_projection(graph);
            let attractor_behaviour = behaviour
                .iter()
                .find(|(_, c)| witness_colour.is_subset(c))
                .unwrap()
                .0
                .clone();
            result.push((attractor_states, attractor_behaviour));
        }
        return result;
    }

    // TODO: Parallelism
    pub fn add_component(&self, component: GraphColoredVertices, graph: &SymbolicAsyncGraph) {
        let mut component_classification = HashMap::new();
        let without_sinks = self.filter_sinks(component.clone(), graph);
        let not_sink_params = without_sinks.color_projection(graph);
        let sink_params = component.color_projection(graph).minus(&not_sink_params);
        if !sink_params.is_empty() {
            component_classification.insert(Behaviour::Stability, sink_params);
        }
        if !not_sink_params.is_empty() {
            let mut disorder = graph.empty_colors().clone();
            for variable in graph.network().graph().variable_ids() {
                let found_first_successor = &graph.has_any_post(variable, &without_sinks);
                for next_variable in graph.network().graph().variable_ids() {
                    if next_variable == variable {
                        continue;
                    }
                    let found_second_successor =
                        &graph.has_any_post(next_variable, &found_first_successor);
                    disorder = disorder.union(&found_second_successor.color_projection(graph));
                }
            }
            let cycle = without_sinks.color_projection(graph).minus(&disorder);
            if !cycle.is_empty() {
                println!("Found cycle: {}", cycle.cardinality());
                component_classification.insert(Behaviour::Oscillation, cycle.clone());
                self.push(Behaviour::Oscillation, cycle);
            }
            if !disorder.is_empty() {
                println!("Found disorder: {}", disorder.cardinality());
                component_classification.insert(Behaviour::Disorder, disorder.clone());
                self.push(Behaviour::Disorder, disorder);
            }
        }
        {
            let mut attractors = self.attractors.lock().unwrap();
            attractors.push((component, component_classification));
        }
    }

    fn push(&self, behaviour: Behaviour, params: GraphColors) {
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

    // TODO: Parallelism
    /// Remove all sink states from the given component (and push them into the classifier).
    fn filter_sinks(
        &self,
        component: GraphColoredVertices,
        graph: &SymbolicAsyncGraph,
    ) -> GraphColoredVertices {
        let mut is_not_sink = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            let has_successor = &graph.has_any_post(variable, &component);
            if !has_successor.is_empty() {
                is_not_sink = is_not_sink.union(has_successor);
            }
        }
        let is_sink = component
            .color_projection(graph)
            .minus(&is_not_sink.color_projection(graph));
        if !is_sink.is_empty() {
            self.push(Behaviour::Stability, is_sink);
        }
        return is_not_sink;
    }
}
