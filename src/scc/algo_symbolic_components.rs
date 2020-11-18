use crate::scc::algo_symbolic_reach::{guarded_reach_bwd, guarded_reach_fwd};
use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_std::param_graph::Params;
use std::option::Option::Some;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn components<F>(
    graph: &SymbolicAsyncGraph,
    progress: &ProgressTracker,
    cancelled: &AtomicBool,
    on_component: F,
) where
    F: Fn(GraphColoredVertices) -> () + Send + Sync,
{
    crossbeam::thread::scope(|scope| {
        println!("Start detecting sinks");
        let mut is_not_sink = graph.empty_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            if cancelled.load(Ordering::SeqCst) {
                return ();
            }
            let has_successor = &graph.has_any_post(variable, graph.unit_vertices());
            is_not_sink = is_not_sink.union(has_successor);
        }
        let mut is_sink = graph.unit_vertices().minus(&is_not_sink);
        let sinks = is_sink.clone();
        // Now we have to report sinks, but we have to satisfy that every reported set has only one component:
        while !is_sink.is_empty() {
            let to_report = is_sink.pivots();
            is_sink = is_sink.minus(&to_report);
            on_component(to_report);
        }

        println!("Sinks detected");

        if cancelled.load(Ordering::SeqCst) {
            return ();
        }

        let can_reach_sink =
            guarded_reach_bwd(&graph, &sinks, &graph.unit_vertices(), cancelled, progress);

        if cancelled.load(Ordering::SeqCst) {
            return ();
        }

        let initial = graph.unit_vertices().minus(&can_reach_sink);

        if initial.is_empty() {
            return ();
        }

        let mut queue: Vec<(f64, GraphColoredVertices)> = Vec::new();
        queue.push((initial.cardinality(), initial));

        while let Some((universe_cardinality, universe)) = queue.pop() {
            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            println!(
                "Universe cardinality: {} Remaining work queue: {}",
                universe_cardinality,
                queue.len()
            );
            let remaining: f64 = queue.iter().map(|(a, _)| *a).sum::<f64>() + universe_cardinality;
            progress.update_remaining(remaining);
            println!("Look for pivots...");
            let pivots = universe.pivots();
            println!("Pivots cardinality: {}", pivots.cardinality());
            let forward = guarded_reach_fwd(&graph, &pivots, &universe, cancelled, progress);

            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            let component_with_pivots =
                guarded_reach_bwd(&graph, &pivots, &forward, cancelled, progress);

            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            let reachable_terminals = forward.minus(&component_with_pivots);

            let leaves_current = reachable_terminals.color_projection();
            let is_terminal = graph.unit_colors().minus(&leaves_current);

            if !is_terminal.is_empty() {
                let terminal = component_with_pivots.intersect_colors(&is_terminal);
                scope.spawn(|_| {
                    on_component(terminal);
                });
            }

            let basins_of_reachable_terminals =
                guarded_reach_bwd(&graph, &forward, &universe, cancelled, progress);

            if cancelled.load(Ordering::SeqCst) {
                return ();
            }

            let unreachable_terminals = universe.minus(&basins_of_reachable_terminals);

            if !leaves_current.is_empty() {
                queue.push((reachable_terminals.cardinality(), reachable_terminals));
            }
            if !unreachable_terminals.is_empty() {
                queue.push((unreachable_terminals.cardinality(), unreachable_terminals));
            }
        }

        println!("Main component loop done...");
    })
    .unwrap();
}
