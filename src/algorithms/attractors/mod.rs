use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use crate::algorithms::non_constant_variables;
use crate::algorithms::reachability::{bwd, fwd_step};

/// **(internal)** Implements interleaved transition guided reduction. This procedure attempts
/// to eliminate a large portion of non-attractor states quickly. The module itself is private,
/// as it implements internal mechanisms used by the algorithm. The main procedure is provided
/// here in the `attractors` module.
///
/// The technical implementation of the reduction lies in having multiple processes that can run
/// in an interleaving fashion, such that a process "scheduler" will always pick `N` "lightest"
/// processes (in terms of the symbolic representation size) and run a single step of each
/// process in parallel. When a process eliminates some states, this change is propagated
/// to all remaining processes.
mod itgr;

pub async fn transition_guided_reduction(
    stg: &SymbolicAsyncGraph,
    fork_limit: usize
) -> GraphColoredVertices {
    itgr::schedule_reductions(stg.clone(), fork_limit).await
}

/// Forward-backward symbolic SCC detection algorithm modified to detect attractors (bottom SCCs).
///
/// The algorithm can be parametrised by two callbacks:
///  - First is called when an attractors is found. The argument represents the attractor set.
///  - Second callback is called when a set of states is eliminated. This set may or may not
///    contain an attractor (however, if it does, the attractor is already reported).
pub async fn attractors<OnAttractor, OnEliminated>(
    stg: &SymbolicAsyncGraph,
    set: &GraphColoredVertices,
    on_attractor: OnAttractor,
    on_eliminated: OnEliminated
) where
    OnAttractor: Fn(GraphColoredVertices) + Send + Sync,
    OnEliminated: Fn(GraphColoredVertices) + Send + Sync,
{
    let root_stg = stg;

    // Restricted STG containing only the remaining vertices.
    let mut active_stg = root_stg.restrict(set);
    while !active_stg.unit_colored_vertices().is_empty() {
        // Compute variables that can still perform some transitions within the remaining graph.
        let active_variables = non_constant_variables(&active_stg);

        // Pick a (colored) vertex and compute the backward-reachable basin.
        let pivot = active_stg.unit_colored_vertices().pick_vertex();
        let pivot_basin = bwd(&active_stg, &pivot, &active_variables).await;

        // Compute the rest of the pivot's SCC, stopping if it is not terminal.
        let mut pivot_component = pivot.clone();
        while let Some(successors) = fwd_step(&active_stg, &pivot_component, &active_variables).await {
            let non_terminal = successors.minus(&pivot_basin);
            if !non_terminal.is_empty() {
                pivot_component = pivot_component.minus_colors(&non_terminal.colors());
            }
        }

        // If there is something remaining in the pivot component, report it as attractor.
        if !pivot_component.is_empty() {
            on_attractor(pivot_component);
        }

        // Further restrict the STG by removing the current basin.
        active_stg = active_stg.restrict(&active_stg.unit_colored_vertices().minus(&pivot_basin));
        on_eliminated(pivot_basin);
    }
}
