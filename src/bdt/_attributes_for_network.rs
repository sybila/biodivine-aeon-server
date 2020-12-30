use crate::bdt::{BDT, Attribute, BifurcationFunction};
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_std::param_graph::Params;

impl BDT {

    pub fn new_from_graph(classes: BifurcationFunction, graph: &SymbolicAsyncGraph) -> BDT {
        let mut attributes = Vec::new();
        attributes_for_network_inputs(graph, &mut attributes);
        attributes_for_constant_parameters(graph, &mut attributes);
        attributes_for_missing_constraints(graph, &mut attributes);
        attributes_for_implicit_function_tables(graph, &mut attributes);
        attributes_for_explicit_function_tables(graph, &mut attributes);
        let attributes = attributes.into_iter()
            .filter(|a| {
                let is_not_empty = !a.positive.is_empty() && !a.negative.is_empty();
                let is_not_empty = is_not_empty && !a.positive.intersect(graph.unit_colors()).is_empty();
                let is_not_empty = is_not_empty && !a.negative.intersect(graph.unit_colors()).is_empty();
                is_not_empty
            })
            .collect();
        BDT::new(classes, attributes)
    }

}

/// **(internal)** Construct basic attributes for all input variables.
fn attributes_for_network_inputs(graph: &SymbolicAsyncGraph, out: &mut Vec<Attribute>) {
    for v in graph.network().variables() {
        // v is input if it has no update function and no regulators
        let is_input = graph.network().regulators(v).is_empty();
        let is_input = is_input && graph.network().get_update_function(v).is_none();
        if is_input {
            let bdd = graph.symbolic_context().mk_implicit_function_is_true(v, &vec![]);
            out.push(Attribute {
                name: graph.network().get_variable_name(v).clone(),
                negative: graph.empty_colors().copy(bdd.not()),
                positive: graph.empty_colors().copy(bdd),
            })
        }
    }
}

/// **(internal)** Construct basic attributes for all constant parameters of the network.
fn attributes_for_constant_parameters(graph: &SymbolicAsyncGraph, out: &mut Vec<Attribute>) {
    for p in graph.network().parameters() {
        if graph.network()[p].get_arity() == 0 {    // Parameter is a constant
            let bdd = graph.symbolic_context().mk_uninterpreted_function_is_true(p, &vec![]);
            out.push(Attribute {
                name: graph.network()[p].get_name().clone(),
                negative: graph.empty_colors().copy(bdd.not()),
                positive: graph.empty_colors().copy(bdd),
            })
        }
    }
}

/// **(internal)** If some regulation has a missing static constraint, try to build it
/// and add it as an attribute.
fn attributes_for_missing_constraints(graph: &SymbolicAsyncGraph, out: &mut Vec<Attribute>) {
    let network = graph.network();
    let context = graph.symbolic_context();
    for reg in graph.network().as_graph().regulations() {
        // This is straight up copied from static constraint analysis in lib-param-bn.
        // For more context, go there.
        let target = reg.get_target();
        let update_function = network.get_update_function(target);
        let fn_is_true = if let Some(function) = update_function {
            context.mk_fn_update_true(function)
        } else {
            context.mk_implicit_function_is_true(target, &network.regulators(target))
        };
        let fn_is_false = fn_is_true.not();
        let regulator: usize = reg.get_regulator().into();
        let regulator = context.state_variables()[regulator];
        let regulator_is_true = context.mk_state_variable_is_true(reg.get_regulator());
        let regulator_is_false = context.mk_state_variable_is_true(reg.get_regulator()).not();

        if !reg.is_observable() {
            let fn_x1_to_1 = fn_is_true.and(&regulator_is_true).var_project(regulator);
            let fn_x0_to_1 = fn_is_true.and(&regulator_is_false).var_project(regulator);
            let observability = fn_x1_to_1.xor(&fn_x0_to_1).project(&context.state_variables());

            out.push(Attribute {
                name: format!(
                    "{} observable in {}",
                    network.get_variable_name(reg.get_regulator()),
                    network.get_variable_name(reg.get_target()),
                ),
                negative: graph.empty_colors().copy(observability.not()),
                positive: graph.empty_colors().copy(observability)
            });
        }

        if reg.get_monotonicity().is_none() {
            let fn_x1_to_0 = fn_is_false.and(&regulator_is_true).var_project(regulator);
            let fn_x0_to_1 = fn_is_true.and(&regulator_is_false).var_project(regulator);
            let non_activation = fn_x0_to_1.and(&fn_x1_to_0).project(&context.state_variables());

            let fn_x0_to_0 = fn_is_false.and(&regulator_is_false).var_project(regulator);
            let fn_x1_to_1 = fn_is_true.and(&regulator_is_true).var_project(regulator);
            let non_inhibition = fn_x0_to_0.and(&fn_x1_to_1).project(&context.state_variables());

            out.push(Attribute {
                name: format!(
                    "{} activation in {}",
                    network.get_variable_name(reg.get_regulator()),
                    network.get_variable_name(reg.get_target()),
                ),
                positive: graph.empty_colors().copy(non_activation.not()),
                negative: graph.empty_colors().copy(non_activation),
            });

            out.push(Attribute {
                name: format!(
                    "{} inhibition in {}",
                    network.get_variable_name(reg.get_regulator()),
                    network.get_variable_name(reg.get_target()),
                ),
                positive: graph.empty_colors().copy(non_inhibition.not()),
                negative: graph.empty_colors().copy(non_inhibition),
            });
        }
    }
}

/// **(internal)** Make an explicit attributes (like `f[1,0,1] = 1`) for every implicit update
/// function row in the network.
fn attributes_for_implicit_function_tables(graph: &SymbolicAsyncGraph, out: &mut Vec<Attribute>) {
    for v in graph.network().variables() {
        let is_implicit_function = graph.network().get_update_function(v).is_none();
        let is_implicit_function = is_implicit_function && !graph.network().regulators(v).is_empty();
        if is_implicit_function {
            let table = graph.symbolic_context().get_implicit_function_table(v);
            for (ctx, var) in table {
                let bdd = graph.symbolic_context().bdd_variable_set().mk_var(var);
                let ctx: Vec<String> = ctx.into_iter()
                    .zip(graph.network().regulators(v))
                    .map(|(b, r)| {
                        format!("{}{}", if b { "" } else { "¬" }, graph.network().get_variable_name(r))
                    }).collect();
                let name = format!("{}{:?}", graph.network().get_variable_name(v), ctx);
                out.push(Attribute {
                    name: name.replace("\"", ""),
                    negative: graph.mk_empty_colors().copy(bdd.not()),
                    positive: graph.mk_empty_colors().copy(bdd),
                });
            }
        }
    }
}

/// **(internal)** Make an explicit argument for every explicit parameter function row in the network.
fn attributes_for_explicit_function_tables(graph: &SymbolicAsyncGraph, out: &mut Vec<Attribute>) {
    for p in graph.network().parameters() {
        let parameter = graph.network().get_parameter(p);
        if parameter.get_arity() > 0 {
            let table = graph.symbolic_context().get_explicit_function_table(p);
            let arg_names = (0..parameter.get_arity()).map(|i| format!("x{}", i+1)).collect::<Vec<_>>();
            for (ctx, var) in table {
                let bdd = graph.symbolic_context().bdd_variable_set().mk_var(var);
                let ctx: Vec<String> = ctx.into_iter()
                    .zip(&arg_names)
                    .map(|(b, r)| {
                        format!("{}{}", if b { "" } else { "¬" }, r)
                    }).collect();
                let name = format!("{}{:?}", parameter.get_name(), ctx);
                out.push(Attribute {
                    name: name.replace("\"", ""),
                    negative: graph.mk_empty_colors().copy(bdd.not()),
                    positive: graph.mk_empty_colors().copy(bdd),
                });
            }
        }
    }
}