use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn guarded_reach_fwd(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    cancelled: &AtomicBool,
    progress: &ProgressTracker,
) -> GraphColoredVertices {
    let mut result = initial.clone();
    let mut frontier = initial.clone();

    while !frontier.is_empty() {
        if cancelled.load(Ordering::SeqCst) {
            return result; // result is incorrect, but we are cancelled so we don't care...
        }

        progress.update_last_wave(frontier.cardinality());

        let mut new_frontier = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            if cancelled.load(Ordering::SeqCst) {
                return result; // result is incorrect, but we are cancelled so we don't care...
            }
            let successors = graph.post(variable, &frontier, guard);
            new_frontier = new_frontier.union(&successors.minus(&result));
            result = result.union(&successors);
        }
        frontier = new_frontier;
    }

    progress.update_last_wave(0.0);
    return result;
}

pub fn guarded_reach_bwd(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    cancelled: &AtomicBool,
    progress: &ProgressTracker,
) -> GraphColoredVertices {
    let mut result = initial.clone();
    let mut frontier = initial.clone();

    while !frontier.is_empty() {
        if cancelled.load(Ordering::SeqCst) {
            return result; // result is incorrect, but we are cancelled so we don't care...
        }

        progress.update_last_wave(frontier.cardinality());

        let mut new_frontier = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            if cancelled.load(Ordering::SeqCst) {
                return result; // result is incorrect, but we are cancelled so we don't care...
            }
            let predecessors = graph.pre(variable, &frontier, guard);
            new_frontier = new_frontier.union(&predecessors.minus(&result));
            result = result.union(&predecessors);
        }
        frontier = new_frontier;
    }

    progress.update_last_wave(0.0);
    return result;
}
