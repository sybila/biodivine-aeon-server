use crate::scc::algo_effectively_constant::remove_effectively_constant_states;
use crate::scc::algo_symbolic_reach::{guarded_reach_bwd, guarded_reach_fwd};
use crate::scc::ProgressTracker;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_std::param_graph::Params;
use std::io;
use std::io::Write;
use std::option::Option::Some;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn prune_sources(
    graph: &SymbolicAsyncGraph,
    set: GraphColoredVertices,
) -> GraphColoredVertices {
    let start = set.cardinality();
    let mut result = set;
    println!("Pruning sources...");
    loop {
        let mut found_something = false;
        for v in graph.network().graph().variable_ids() {
            let v_true = graph.state_variable_true(v);
            let v_false = graph.state_variable_false(v);
            let jumps_false_to_true = graph.has_any_post(v, &result.intersect(&v_false));
            let jumps_true_to_false = graph.has_any_post(v, &result.intersect(&v_true));
            /*
               Now we are looking for colors that only jump one way: for these, we can fix the value
               because they will never flip back...
            */
            let colors_false_to_true = jumps_false_to_true.color_projection();
            let colors_true_to_false = jumps_true_to_false.color_projection();
            let colors_that_jump_both_ways = colors_true_to_false.intersect(&colors_true_to_false);
            let colors_that_jump_only_one_way = (colors_false_to_true.union(&colors_true_to_false))
                .minus(&colors_that_jump_both_ways);

            if !colors_that_jump_only_one_way.is_empty() {
                found_something = true;
                let one_way_colors_false_to_true =
                    colors_false_to_true.intersect(&colors_that_jump_only_one_way);
                let one_way_colors_true_to_false =
                    colors_true_to_false.intersect(&colors_that_jump_only_one_way);
                result = result.minus(&v_true.intersect_colors(&one_way_colors_true_to_false));
                result = result.minus(&v_false.intersect_colors(&one_way_colors_false_to_true));
            }
            println!("{:?} -> {}", v, colors_that_jump_only_one_way.cardinality());
        }
        println!(
            "Keeping {}/{} ({:+e}%, nodes result({}))",
            result.cardinality(),
            start,
            (result.cardinality() / start) * 100.0,
            result.clone().into_bdd().size()
        );
        if !found_something {
            return result;
        }
    }
}

pub fn components<F>(
    graph: &SymbolicAsyncGraph,
    progress: &ProgressTracker,
    cancelled: &AtomicBool,
    on_component: F,
) where
    F: Fn(GraphColoredVertices) -> () + Send + Sync,
{
    crossbeam::thread::scope(|scope| {
        println!("Detect eventually stable...");
        // TODO: This is not correct, because for parametrisations can_move will never be empty...
        /*let mut without_fixed = graph.unit_vertices().clone();
        for variable in graph.network().graph().variable_ids() {
            let true_states = graph.state_variable_true(variable).intersect(&without_fixed);
            let false_states = graph.state_variable_false(variable).intersect(&without_fixed);
            let can_move_true = graph.has_any_post(variable, &true_states);
            let can_move_false = graph.has_any_post(variable, &false_states);
            if can_move_true.is_empty() {
                // Every transition for this variable is 0 -> 1, hence states that have this
                // transition enabled cannot be in attractor because it would never reverse...
                without_fixed = without_fixed.minus(&can_move_false)

                // At this point, we also know that the two sets (true states and false states)
                // are independent and can be processed in parallel! We should use that! TODO...
            }
            if can_move_false.is_empty() {
                without_fixed = without_fixed.minus(&can_move_true)
            }
        }
        println!("Fixed {}/{}", without_fixed.cardinality(), graph.unit_vertices().cardinality());*/

        println!("Start detecting sinks");

        let mut can_be_sink = graph.unit_vertices().clone(); // intentionally use all vertices
                                                             //panic!("");
        for variable in graph.network().graph().variable_ids() {
            print!("{:?}...", variable);
            io::stdout().flush().unwrap();
            if cancelled.load(Ordering::SeqCst) {
                return ();
            }
            let has_successor = &graph.has_any_post(variable, graph.unit_vertices());
            can_be_sink = can_be_sink.minus(has_successor);
        }
        println!();

        let mut is_sink = can_be_sink.clone();
        /*for sink in is_sink.state_projection(graph).states(graph) {
            let mut valuations = Vec::new();
            for (i_v, v) in graph.network().graph().variable_ids().enumerate() {
                let name = graph.network().graph().get_variable(v).get_name();
                valuations.push((name.clone(), sink.get(i_v)));
            }
            let sink_colors = is_sink.intersect(&graph.vertex(sink.clone())).color_projection();
            let sink_remaining = is_sink.minus(&graph.vertex(sink.clone())).intersect_colors(&sink_colors);
            let sink_rank = if sink_remaining.is_empty() { 1 } else { 2 };

            println!("========================= Sink state (Rank {}) {:?} =========================", sink_rank, sink.values());
            println!("{:?}", valuations);
            println!("========================= Witness network =========================");
            let witness = graph.make_witness(&sink_colors);
            println!("{}", witness.to_string());
        }*/
        let sinks = is_sink.clone();
        // Now we have to report sinks, but we have to satisfy that every reported set has only one component:
        while !is_sink.is_empty() {
            let to_report = is_sink.pivots();
            is_sink = is_sink.minus(&to_report);
            on_component(to_report);
        }

        println!("Sinks detected: {}", sinks.cardinality());

        /*let has_successors: Vec<GraphColoredVertices> = graph.network().graph().variable_ids()
            .collect::<Vec<VariableId>>()
            .into_par_iter()
            .map(|variable: VariableId| {
                graph.has_any_post(variable, graph.unit_vertices())
            })
            .collect();
        let has_successors = par_fold(has_successors, |a, b| a.union(b));*/

        let not_constant = remove_effectively_constant_states(graph, graph.unit_vertices().clone());
        println!(
            "Not constant: {}/{}",
            not_constant.cardinality(),
            graph.unit_vertices().cardinality()
        );

        if cancelled.load(Ordering::SeqCst) {
            return ();
        }

        let can_reach_sink = guarded_reach_bwd(&graph, &sinks, &not_constant, cancelled, progress);

        if cancelled.load(Ordering::SeqCst) {
            return ();
        }

        let initial = not_constant.minus(&can_reach_sink);

        println!("Initial: {}", initial.cardinality());

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
            let backward = guarded_reach_bwd(&graph, &pivots, &universe, cancelled, progress);
            let component_with_pivots =
                guarded_reach_fwd(&graph, &pivots, &backward, cancelled, progress);

            let mut is_terminal = component_with_pivots.color_projection();
            for v in graph.network().graph().variable_ids() {
                let successors = graph
                    .any_post(v, &component_with_pivots)
                    .minus(&component_with_pivots)
                    .color_projection();
                if !successors.is_empty() {
                    is_terminal = is_terminal.minus(&successors);
                }
            }

            if !is_terminal.is_empty() {
                let terminal = component_with_pivots.intersect_colors(&is_terminal);
                scope.spawn(|_| {
                    on_component(terminal);
                });
            }

            let remaining = universe.minus(&backward);
            if !remaining.is_empty() {
                queue.push((remaining.cardinality(), remaining));
            }

            /*
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
            }*/
        }

        println!("Main component loop done...");
    })
    .unwrap();
}
