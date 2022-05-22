use std::cmp::{max, min};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::{BooleanNetwork, ParameterId, RegulatoryGraph, VariableId};
use std::convert::TryFrom;
use std::io::Read;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use rocket::form::validate::Contains;
use text_io::read;
use biodivine_aeon_server::algorithms::attractors::transition_guided_reduction;
use biodivine_aeon_server::algorithms::reachability::bwd;

#[tokio::main]
async fn main() {
    //let mut buffer = String::new();
    //std::io::stdin().read_to_string(&mut buffer).unwrap();

    let args = std::env::args().collect::<Vec<_>>();
    let buffer = std::fs::read_to_string(args[1].as_str()).unwrap();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    let model = inline_inputs(model);
    println!("Model loaded. {} variables and {} parameters.", model.num_vars(), model.num_parameters());

    let graph = SymbolicAsyncGraph::new(model.clone()).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrisations: {}",
        graph.unit_colors().approx_cardinality()
    );
    println!(
        "State space: {}",
        graph.unit_colored_vertices().approx_cardinality()
    );

    let mut params = model.parameters().collect::<Vec<_>>();
    params.sort_by_cached_key(|p| find_dependent_variables(&model, *p).len());
    params.reverse();

    find_sinks_approx(&graph.clone(), &graph.clone(), &params);

    //let mut sinks = graph.mk_unit_colored_vertices();
    /*for var in model.variables() {
        let can_post = graph.var_can_post(var, &sinks);
        sinks = sinks.minus(&can_post);
        println!("Applied {:?}, result is {} / {}", var, sinks.approx_cardinality(), sinks.symbolic_size());
    }*/

    /*let mut candidates = model.variables().collect::<Vec<_>>();
    while !candidates.is_empty() {
        let mut best = (usize::MAX, model.variables().next().unwrap());
        for var in &candidates {
            let can_post = graph.var_can_post(*var, &sinks);
            let result = sinks.minus(&can_post);
            if result.symbolic_size() < best.0 {
                best = (result.symbolic_size(), *var);
            }
        }
        let best = best.1;
        let index = candidates.iter().position(|x| *x == best).unwrap();
        candidates.remove(index);
        let can_post = graph.var_can_post(best, &sinks);
        sinks = sinks.minus(&can_post);
        println!("Applied {:?} ({}), result is {} / {}", best, candidates.len(), sinks.approx_cardinality(), sinks.symbolic_size());
    }*/
    /*let candidates = model.variables().collect::<Vec<_>>();
    let sink_list = find_sinks(&graph, candidates, graph.mk_unit_colored_vertices(), 100_000);

    for sink_set in sink_list {
        println!("Sinks: {} / {}", sink_set.symbolic_size(), sink_set.approx_cardinality());
    }*/

    /*
    println!("Sinks: {}", sinks.approx_cardinality());
    let variables = graph.as_network().variables().collect::<Vec<_>>();
    let basin = bwd(&graph, &sinks, &variables).await;
    println!("Basin: {} / {}", basin.approx_cardinality(), basin.symbolic_size());
    let not_basin = graph.unit_colored_vertices().minus(&basin);
    println!("Not basin: {} / {}", not_basin.approx_cardinality(), not_basin.symbolic_size());*/
}

fn find_sinks_approx(
    over_approx: &SymbolicAsyncGraph,
    under_approx: &SymbolicAsyncGraph,
    parameters: &[ParameterId],
) -> (usize, Vec<GraphColoredVertices>) {
    let variables = over_approx.as_network().variables().collect::<Vec<_>>();
    if parameters.is_empty() {
        let under_sinks = find_sinks_greedy(&under_approx, variables.clone(), under_approx.mk_unit_colored_vertices());
        let over_sinks = find_sinks_greedy(&over_approx, variables.clone(), over_approx.mk_unit_colored_vertices());
        println!("Under: {} / {}", under_sinks.approx_cardinality(), under_sinks.symbolic_size());
        println!("Over: {} / {}", over_sinks.approx_cardinality(), over_sinks.symbolic_size());
        println!("All: {} / {}", under_approx.unit_colored_vertices().approx_cardinality(), under_approx.unit_colored_vertices().symbolic_size());
        return (under_sinks.symbolic_size(), vec![under_sinks]);
    } else {
        let parameter = parameters[0];
        let (limit, sink_candidates) = find_sinks_approx(
            &over_approx.param_over_approximation(parameter),
            &under_approx.param_under_approximation(parameter),
            &parameters[1..]
        );

        /*let limit = sink_candidates.iter()
            .map(|it| it.symbolic_size())
            .max()
            .unwrap();*/

        let mut new_candidates = Vec::new();
        for candidates in sink_candidates {
            let under_sinks = find_sinks_greedy(&under_approx, find_dependent_variables(under_approx.as_network(), parameter), candidates.clone());
            //let over_sinks = find_sinks_greedy(&over_approx, variables.clone(), candidates.clone());
            println!("Under: {} / {}", under_sinks.approx_cardinality(), under_sinks.symbolic_size());
            //println!("Over: {} / {}", over_sinks.approx_cardinality(), over_sinks.symbolic_size());
            println!("All: {} / {}", under_approx.unit_colored_vertices().approx_cardinality(), under_approx.unit_colored_vertices().symbolic_size());
            if under_sinks.symbolic_size() > limit {
                let (_, bdd_var) = under_approx.symbolic_context()
                    .get_explicit_function_table(parameter)
                    .into_iter()
                    .next()
                    .unwrap();
                let true_candidates = under_approx.unit_colored_vertices().copy(
                    under_sinks.as_bdd().var_select(bdd_var, true)
                );
                let false_candidates = under_approx.unit_colored_vertices().copy(
                    under_sinks.as_bdd().var_select(bdd_var, false)
                );
                new_candidates.push(true_candidates);
                new_candidates.push(false_candidates);
            } else {
                new_candidates.push(under_sinks);
            }
        }

        println!("Finished {}. Candidate sets: {}", parameters.len(), new_candidates.len());
        return (limit, new_candidates);
    }
}

fn find_dependent_variables(bn: &BooleanNetwork, param: ParameterId) -> Vec<VariableId> {
    let mut result = Vec::new();
    for var in bn.variables() {
        if let Some(update) = bn.get_update_function(var) {
            let params = update.collect_parameters();
            if params.contains(&param) {
                result.push(var);
            }
        }
    }
    result
}

fn find_sinks_greedy(
    stg: &SymbolicAsyncGraph,
    mut variables: Vec<VariableId>,
    mut sinks: GraphColoredVertices
) -> GraphColoredVertices {
    /*let mut empty = Vec::new();
    for var in &variables {
        let can_post = stg.var_can_post(*var, &sinks);
        println!("Var {:?} can post: {}", var, can_post.approx_cardinality());
        if can_post.is_empty() {
            empty.push(*var)
        }
        if sinks.is_subset(&can_post) {
            println!("Found a universal variable.");
            return stg.mk_empty_vertices();
        }
    }
    variables.retain(|it| !empty.contains(it));*/
    while !variables.is_empty() {
        let best = {
            if variables.len() <= 10 {
                variables.iter().next().unwrap().clone()
            } else {
                let mut best = (usize::MAX, *variables.iter().next().unwrap());
                for var in &variables {
                    let can_post = stg.var_can_post(*var, &sinks);
                    let result = sinks.minus(&can_post);
                    if result.symbolic_size() < best.0 {
                        best = (result.symbolic_size(), *var);
                    }
                }
                best.1
            }
        };
        let index = variables.iter().position(|x| *x == best).unwrap();
        variables.remove(index);
        let can_post = stg.var_can_post(best, &sinks);
        sinks = sinks.minus(&can_post);
        println!("Applied {:?} ({}), result is {} / {}", best, variables.len(), sinks.approx_cardinality(), sinks.symbolic_size());
    }

    sinks
}

fn find_sinks(
    stg: &SymbolicAsyncGraph,
    mut variables: Vec<VariableId>,
    mut states: GraphColoredVertices,
    limit: usize,
) -> Vec<GraphColoredVertices> {
    if variables.is_empty() {
        println!("Found sinks: {} / {}", states.symbolic_size(), states.approx_cardinality());
        return vec![states];
    }

    if states.symbolic_size() < limit {
        // Pick the best restriction variable based on the impact on the symbolic set size.
        let mut best = (usize::MAX, *variables.iter().next().unwrap());
        for var in &variables {
            let can_post = stg.var_can_post(*var, &states);
            let result = states.minus(&can_post);
            if result.symbolic_size() < best.0 {
                best = (result.symbolic_size(), *var);
            }
        }
        let best = best.1;
        let index = variables.iter().position(|x| *x == best).unwrap();
        variables.remove(index);
        let can_post = stg.var_can_post(best, &states);
        states = states.minus(&can_post);
        println!("Applied {:?} ({}), result is {} / {}", best, variables.len(), states.approx_cardinality(), states.symbolic_size());
        return find_sinks(stg, variables, states, limit);
    }

    println!("Forking, because set is {}.", states.symbolic_size());

    /*for (i, p_var) in stg.symbolic_context().parameter_variables().iter().enumerate() {
        let select_true = states.as_bdd().var_select(*p_var, true);
        let select_false = states.as_bdd().var_select(*p_var, false);

        println!("Variable {} / {:?} produces {} | {} = {}", i, p_var, select_true.size(), select_false.size(), select_true.size() + select_false.size());

    }*/

    /*for parameter in stg.as_network().parameters() {
        let select_true = stg.param_select_vertices(&states, (parameter, true));
        let select_false = stg.param_select_vertices(&states, (parameter, false));

        println!("Parameter {:?} produces {} | {} = {}",
                 parameter,
                 select_true.symbolic_size(),
                 select_false.symbolic_size(),
                 select_true.symbolic_size() + select_false.symbolic_size());
    }

    let n: usize = read!();
    println!("Selected: {}", n);*/

    /*let bdd_var = stg.symbolic_context().parameter_variables()[n];
    let true_fork = stg.empty_vertices().copy(
        states.as_bdd().var_select(bdd_var, true)
    );
    let false_fork = stg.empty_vertices().copy(
        states.as_bdd().var_select(bdd_var, false)
    );

    let mut true_result = find_sinks(stg, variables.clone(), true_fork);
    let false_result = find_sinks(stg, variables.clone(), false_fork);*/

    let parameter = pick_best_fork(stg, &states);//stg.as_network().parameters().nth(n).unwrap();

    if let Some(parameter) = parameter {
        let mut true_result = {
            let true_stg = stg.param_select((parameter, true));
            let true_set = stg.param_select_vertices(&states, (parameter, true));
            let new_limit = if true_set.symbolic_size() > limit {
                println!("Increasing limit to {}", 2 * true_set.symbolic_size());
                2 * true_set.symbolic_size()
            } else {
                limit
            };
            find_sinks(&true_stg, variables.clone(), true_set, new_limit)
        };
        println!("States before propagation: {} / {}", states.symbolic_size(), states.approx_cardinality());
        for result in &true_result {
            println!("Re-searching result {} / {}", result.symbolic_size(), result.approx_cardinality());
            let result_flipped = stg.param_lift_vertices(result, (parameter, false));
            let found = find_sinks(stg, variables.clone(), result_flipped, usize::MAX);
            for f in found {
                println!("> Found {} / {}", f.symbolic_size(), f.approx_cardinality());
                states = states.minus(&f);
            }
        }
        println!("States after propagation: {} / {}", states.symbolic_size(), states.approx_cardinality());
        let false_result = {
            let false_stg = stg.param_select((parameter, false));
            let false_set = stg.param_select_vertices(&states, (parameter, false));
            let new_limit = if false_set.symbolic_size() > limit {
                println!("Increasing limit to {}", 2 * false_set.symbolic_size());
                2 * false_set.symbolic_size()
            } else {
                limit
            };
            find_sinks(&false_stg, variables.clone(), false_set, new_limit)
        };
        true_result.extend(false_result.into_iter());
        true_result
    } else {
        println!("Increasing limit to {}", 2 * limit);
        find_sinks(stg, variables, states, 2 * limit)
    }
}

fn pick_best_fork(stg: &SymbolicAsyncGraph, states: &GraphColoredVertices) -> Option<ParameterId> {
    println!("Picking fork...");
    let mut best: Option<(ParameterId, f64)> = None;
    for parameter in stg.as_network().parameters() {
        let select_true = stg.param_select_vertices(&states, (parameter, true));
        let select_false = stg.param_select_vertices(&states, (parameter, false));

        if select_true.symbolic_size() >= states.symbolic_size() {
            continue;
        }
        if select_false.symbolic_size() >= states.symbolic_size() {
            continue;
        }

        let true_size = select_true.symbolic_size();
        let false_size = select_false.symbolic_size();

        // Both errors are in [1..len(states)], so we can give them the same weight.
        let size_error = ((true_size + false_size) / 2) as f64;
        let skew_error = (max(true_size, false_size) - min(true_size, false_size)) as f64;

        let error = (((size_error*size_error) + (skew_error*skew_error)) / 2.0).sqrt();

        if let Some((_, best_error)) = best {
            if error < best_error {
                best = Some((parameter, error));
            }
        } else {
            best = Some((parameter, error));
        }

        /*println!("Parameter {:?} produces {} | {} = {} with error {}",
                 parameter,
                 select_true.symbolic_size(),
                 select_false.symbolic_size(),
                 select_true.symbolic_size() + select_false.symbolic_size(),
                 error
        );*/
    }

    println!("Picked {:?}", best);
    best.map(|(p, _)| p)
}

fn inline_inputs(bn: BooleanNetwork) -> BooleanNetwork {
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
            inlined_rg.add_regulation(
                old_regulator,
                old_target,
                false,
                reg.get_monotonicity()
            ).unwrap();
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
        inlined_bn.add_string_update_function(name.as_str(), old_function.to_string(&bn).as_str()).unwrap();
    }

    inlined_bn
}
