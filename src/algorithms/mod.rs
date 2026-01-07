/// Symbolic classifier that can be used for incremental partitioning of the color space based
/// on some "atomic" features supplied on-the-fly.
pub mod incremental_classifier;

/// Describes a very basic differentiation of asymptotic behaviour within an SCC-closed set
/// as either `stability`, `oscillation` or `disorder`. Provides basic symbolic algorithms
/// for detecting exactly these three types of behaviour.
pub mod asymptotic_behaviour;

/// Reachability provides the main building blocks for more complex graph algorithms. The methods
/// here are specifically designed to fit well into other processes in AEON. For basic tasks,
/// you may be better off with some of the simpler algorithms provided by `SymbolicAsyncGraph`
/// directly.
pub mod reachability;

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
use biodivine_lib_param_bn::VariableId;
pub use symbolic_counter::SymbolicCounter;

/// Identify the `VariableId` objects for which the given `stg` can perform *some* transition.
pub fn non_constant_variables(stg: &SymbolicAsyncGraph) -> Vec<VariableId> {
    stg.variables()
        .filter(|var| {
            !stg.var_can_post(*var, stg.unit_colored_vertices())
                .is_empty()
        })
        .collect()
}
