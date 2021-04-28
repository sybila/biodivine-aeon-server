use crate::scc::algo_stability_analysis::{
    AttractorStabilityData, Stability, StabilityVector, VariableStability,
};
use crate::util::functional::Functional;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{
    GraphColoredVertices, GraphColors, SymbolicAsyncGraph,
};
use biodivine_lib_param_bn::VariableId;
use json::JsonValue;
use std::convert::TryFrom;
use std::ops::{Index, IndexMut};

impl Index<StabilityVector> for VariableStability {
    type Output = Option<GraphColors>;

    fn index(&self, index: StabilityVector) -> &Self::Output {
        let id: usize = index.into();
        &self.0[id]
    }
}

impl IndexMut<StabilityVector> for VariableStability {
    fn index_mut(&mut self, index: StabilityVector) -> &mut Self::Output {
        let id: usize = index.into();
        &mut self.0[id]
    }
}

impl VariableStability {
    /// Add a value for behaviour if not present, otherwise union with current value.
    pub fn push(&mut self, behaviour: StabilityVector, colors: GraphColors) {
        if let Some(current) = self[behaviour].as_mut() {
            *current = colors.union(current);
        } else {
            self[behaviour] = Some(colors);
        }
    }

    /// Convert this stability data to a vector of pairs.
    pub fn to_vec(&self) -> Vec<(StabilityVector, GraphColors)> {
        self.0
            .iter()
            .enumerate()
            .filter_map(|(id, p)| p.clone().map(|p| (id, p)))
            .map(|(i, colors)| (StabilityVector::try_from(i).unwrap(), colors))
            .collect()
    }

    /// Compute stability data for a fixed variable and all available attractors.
    pub fn for_attractors(
        graph: &SymbolicAsyncGraph,
        attractors: &[GraphColoredVertices],
        variable: VariableId,
    ) -> VariableStability {
        let mut stability = VariableStability::default();
        let all_colors = attractors
            .iter()
            .fold(graph.mk_empty_colors(), |a, b| a.union(&b.colors()));
        stability.push(StabilityVector::default(), all_colors);
        for attractor in attractors {
            let attractor_stability =
                AttractorStabilityData::for_attractor(graph, attractor, variable);
            let mut updated_stability = VariableStability::default();
            for (vector, colors) in stability.to_vec() {
                let add_true = attractor_stability.stability_true.intersect(&colors);
                let add_false = attractor_stability.stability_false.intersect(&colors);
                let add_unstable = attractor_stability.unstable.intersect(&colors);
                let remaining = colors
                    .minus(&add_true)
                    .minus(&add_false)
                    .minus(&add_unstable);
                if !add_true.is_empty() {
                    let new_key = vector.add(Stability::True);
                    updated_stability.push(new_key, add_true);
                }
                if !add_false.is_empty() {
                    updated_stability.push(vector.add(Stability::False), add_false);
                }
                if !add_unstable.is_empty() {
                    updated_stability.push(vector.add(Stability::Unstable), add_unstable);
                }
                if !remaining.is_empty() {
                    updated_stability.push(vector, remaining);
                }
            }
            stability = updated_stability;
        }

        stability
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::new_array().apply(|array| {
            for (vector, colors) in self.to_vec() {
                array
                    .push(object! {
                        "vector": vector.export_json(),
                        "colors": colors.approx_cardinality(),
                    })
                    .unwrap();
            }
        })
    }
}
