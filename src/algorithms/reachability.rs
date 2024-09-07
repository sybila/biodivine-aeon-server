use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;

/// Try to compute direct successors of the provided `set` with respect to the given `stg`.
/// The successors are also limited to the given selection of `variables`.
///
/// If there is a variable whose application leads to the discovery of new states, these
/// discovered states are returned. Note that these might not include all state from the
/// original `set`. Otherwise, `None` is returned.
///
/// Keep in mind that you can use `SymbolicAsyncGraph::restrict` to narrow down the scope
/// of the `stg` to some desired sub-graph.
pub async fn fwd_step(
    stg: &SymbolicAsyncGraph,
    set: &GraphColoredVertices,
    variables: &[VariableId],
) -> Option<GraphColoredVertices> {
    for var in variables {
        let step = stg.var_post_out(*var, set);
        if !step.is_empty() {
            return Some(step);
        }
    }
    None
}

/// The same as `fwd_step`, but for predecessors.
pub async fn bwd_step(
    stg: &SymbolicAsyncGraph,
    set: &GraphColoredVertices,
    variables: &[VariableId],
) -> Option<GraphColoredVertices> {
    for var in variables {
        let step = stg.var_pre_out(*var, set);
        if !step.is_empty() {
            return Some(step);
        }
    }
    None
}

/// Compute all forward reachable states from the given `set` within the given `stg` using
/// the provided `variables`.
///
/// You can use `variables` and `SymbolicAsyncGraph::restrict` to limit the scope
/// of the exploration.
pub async fn fwd(
    stg: &SymbolicAsyncGraph,
    set: &GraphColoredVertices,
    variables: &[VariableId],
) -> GraphColoredVertices {
    let mut result = set.clone();
    while let Some(successors) = fwd_step(stg, &result, variables).await {
        result = result.union(&successors);
    }
    result
}

/// Compute all backward reachable states from the given `set` within the given `stg` using
/// the provided `variables`.
///
/// You can use `variables` and `SymbolicAsyncGraph::restrict` to limit the scope
/// of the exploration.
pub async fn bwd(
    stg: &SymbolicAsyncGraph,
    set: &GraphColoredVertices,
    variables: &[VariableId],
) -> GraphColoredVertices {
    let mut result = set.clone();
    while let Some(predecessors) = bwd_step(stg, &result, variables).await {
        result = result.union(&predecessors);
    }
    result
}
