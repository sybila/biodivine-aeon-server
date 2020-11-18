use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use std::io::Write;
use rayon::prelude::*;
use biodivine_lib_param_bn::VariableId;
use crate::scc::algo_par_utils::par_fold;

pub fn guarded_reach_fwd(
    graph: &SymbolicAsyncGraph,
    initial: &GraphColoredVertices,
    guard: &GraphColoredVertices,
    cancelled: &AtomicBool,
    progress: &ProgressTracker,
) -> GraphColoredVertices {
    let mut result = initial.clone();
    let mut frontier = initial.clone();

    println!("Reach fwd...");
    while !frontier.is_empty() {
        if cancelled.load(Ordering::SeqCst) {
            return result; // result is incorrect, but we are cancelled so we don't care...
        }

        progress.update_last_wave(frontier.cardinality());

        println!("{}/{} ({:+e}%)", result.cardinality(), guard.cardinality(), (result.cardinality()/guard.cardinality()) * 100.0);
        print!("{} || ", frontier.cardinality());
        let mut new_frontier = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            print!("{:?}...", variable);
            io::stdout().flush().unwrap();
            if cancelled.load(Ordering::SeqCst) {
                return result; // result is incorrect, but we are cancelled so we don't care...
            }
            let successors = graph.post(variable, &frontier, guard);
            new_frontier = new_frontier.union(&successors.minus(&result));
            result = result.union(&successors);
        }
        println!();
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

    println!("Reach bwd...");
    while !frontier.is_empty() {
        if cancelled.load(Ordering::SeqCst) {
            return result; // result is incorrect, but we are cancelled so we don't care...
        }

        progress.update_last_wave(frontier.cardinality());

        println!("{}/{} ({:+e}%)", result.cardinality(), guard.cardinality(), (result.cardinality()/guard.cardinality()) * 100.0);
        print!("{} || ", frontier.cardinality());
        /*let var_predecessors: Vec<GraphColoredVertices> = graph.network().graph().variable_ids().collect::<Vec<VariableId>>()
            .into_par_iter()
            .map(|variable| {
                let mut predecessors = graph.empty_vertices().clone();
                let mut sub_frontier = graph.pre(variable, &frontier, guard);
                while !sub_frontier.is_empty() {
                    sub_frontier = sub_frontier.minus(&predecessors);
                    predecessors = predecessors.union(&sub_frontier);
                    sub_frontier = graph.pre(variable, &sub_frontier, guard);
                }
                predecessors
            })
            .collect();
        let predecessors = par_fold(var_predecessors, |a, b| a.union(b));
        frontier = predecessors.minus(&result);
        result = result.union(&predecessors);*/
        let mut new_frontier = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            print!("{:?}...", variable);
            io::stdout().flush().unwrap();
            if cancelled.load(Ordering::SeqCst) {
                return result; // result is incorrect, but we are cancelled so we don't care...
            }
            let mut predecessors = graph.pre(variable, &frontier, guard).minus(&result);
            result = result.union(&predecessors);
            new_frontier = new_frontier.union(&predecessors);
            while !predecessors.is_empty() {   // Saturate variable!
                predecessors = graph.pre(variable, &predecessors, guard).minus(&result);
                result = result.union(&predecessors);
                new_frontier = new_frontier.union(&predecessors);
            }
        }
        println!();
        frontier = new_frontier;
    }

    progress.update_last_wave(0.0);
    return result;
}
