/// Symbolic classifier that can be used for incremental partitioning of the color space based
/// on some "atomic" features supplied on-the-fly.
pub mod incremental_classifier;

/// Describes a very basic differentiation of asymptotic behaviour within an SCC-closed set
/// as either `stability`, `oscillation` or `disorder`. Provides basic symbolic algorithms
/// for detecting exactly these three types of behaviour.
pub mod asymptotic_behaviour;

/// Reachability provides the main building blocks for more complex graph algorithms. The methods
/// here are specifically designed to fit well into other processes in AEON. For basic tasks,
/// you may be better of with some of the simpler algorithms provided by `SymbolicAsyncGraph`
/// directly.
pub mod reachability;

/// Symbolic algorithms for detecting attractors. Includes Xie-Beerel forward-backward algorithm
/// and transition guided reduction with parallelism and interleaving.
pub mod attractors;

/// Symbolic algorithms for computing all fixed-point attractors.
///
/// These are generally faster than computing all attractors, but might not always be.
mod fixed_points;

/// **(internal)** Implementation of a `SymbolicCounter` that provides a very basic usage
/// example of `IncrementalClassifier` for counting the number of encounters of a particular
/// member of a symbolic set.
mod symbolic_counter;

/// **(internal)** Provides `AsymptoticBehaviourCounter` that uses `AsymptoticBehaviour` and
/// `IncrementalClassifier` to count the number of occurrences of different types of asymptotic
/// behaviour.
mod asymptotic_behaviour_counter;

/// **(internal)** Provides `AsymptoticBehaviourClassifier` that uses `AsymptoticBehaviour` as
/// features in an `IncrementalClassifier`. It does not count the multiplicity of each behaviour,
/// only remembers whether the behaviour was seen.
mod asymptotic_behaviour_classifier;

// Re-export stuff from private modules to public scope as part of `algorithms` module:

pub use asymptotic_behaviour_classifier::AsymptoticBehaviourClassifier;
pub use asymptotic_behaviour_counter::AsymptoticBehaviourCounter;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::{BooleanNetwork, RegulatoryGraph, VariableId};
pub use fixed_points::FixedPoints;
pub use symbolic_counter::SymbolicCounter;

/// Identify the `VariableId` objects for which the given `stg` can perform *some* transition.
pub fn non_constant_variables(stg: &SymbolicAsyncGraph) -> Vec<VariableId> {
    stg.as_network()
        .variables()
        .filter(|var| {
            !stg.var_can_post(*var, stg.unit_colored_vertices())
                .is_empty()
        })
        .collect()
}

/// Create a new `BooleanNetwork` where every input variable of the source `bn` is transformed
/// into a zero-arity parameter.
pub fn inline_network_inputs(bn: BooleanNetwork) -> BooleanNetwork {
    let mut variables = Vec::new();
    let mut parameters = Vec::new();
    for var in bn.variables() {
        if bn.regulators(var).len() == 0 {
            parameters.push(bn.get_variable_name(var).clone());
        } else {
            variables.push(bn.get_variable_name(var).clone());
        }
    }

    let mut inlined_rg = RegulatoryGraph::new(variables.clone());

    for reg in bn.as_graph().regulations() {
        let old_regulator = bn.get_variable_name(reg.get_regulator());
        let old_target = bn.get_variable_name(reg.get_target());
        if variables.contains(old_regulator) {
            inlined_rg
                .add_regulation(old_regulator, old_target, false, reg.get_monotonicity())
                .unwrap();
        }
    }

    let mut inlined_bn = BooleanNetwork::new(inlined_rg);

    for param in parameters {
        inlined_bn.add_parameter(param.as_str(), 0).unwrap();
    }

    for var in inlined_bn.variables() {
        let name = inlined_bn.get_variable_name(var).clone();
        let old_id = bn.as_graph().find_variable(name.as_str()).unwrap();
        let old_function = bn.get_update_function(old_id).as_ref().unwrap();
        inlined_bn
            .add_string_update_function(name.as_str(), old_function.to_string(&bn).as_str())
            .unwrap();
    }

    inlined_bn
}
