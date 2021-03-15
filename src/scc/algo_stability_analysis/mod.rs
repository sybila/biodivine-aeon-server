use biodivine_lib_param_bn::symbolic_async_graph::{
    GraphColoredVertices, GraphColors, SymbolicAsyncGraph,
};
use biodivine_lib_param_bn::VariableId;
use std::collections::HashMap;

mod _impl_attractor_stability_data;
mod _impl_stability;
mod _impl_stability_vector;
mod _impl_variable_stability;

/// A basic enum which defines the stability of a particular variable in one attractor.
///
/// In such case, a variable can be stable, with either true or false as a value, or unstable.
/// That is, the value of the variable is changing in the attractor.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Stability {
    True,
    False,
    Unstable,
}

/// For a given attractor, this struct stores the parametrised data about stability of a
/// particular variable.
///
/// Essentially, it is a mapping from `Stability` values to `GraphColors`. Since there are only
/// three stability values, it is easy to store them explicitly.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AttractorStabilityData {
    stability_true: GraphColors,
    stability_false: GraphColors,
    unstable: GraphColors,
}

/// Stability vector encodes the possible stability phenotypes of a particular variable in
/// multiple attractors.
///
/// `StabilityVector` is basically a set of `Stability` values. But since there are only three,
/// we can again implement this a bit more concisely.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Ord, PartialOrd, Default)]
pub struct StabilityVector {
    has_true: bool,
    has_false: bool,
    has_unstable: bool,
}

/// For multiple attractors, the stability of a variable is a mapping from possible
/// `StabilityVectors` to `GraphColors`.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct VariableStability([Option<GraphColors>; 8]);

/// All stability data for all variables.
pub type StabilityData = HashMap<VariableId, VariableStability>;

/// Compute all stability data for all variables.
pub fn compute_stability(
    graph: &SymbolicAsyncGraph,
    components: &[GraphColoredVertices],
) -> StabilityData {
    graph
        .as_network()
        .variables()
        .map(|id| (id, VariableStability::for_attractors(graph, components, id)))
        .collect()
}
