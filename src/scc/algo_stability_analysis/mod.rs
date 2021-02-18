use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;
use json::JsonValue;

/// Given a set of model sinks, this computes which variables have a fixed constant value
/// and which variables can depend on parametrisation, while also including the proportion
/// in which they appear with either value (or both).
pub fn stability_analysis(
    graph: &SymbolicAsyncGraph,
    sinks: &GraphColoredVertices,
    variable: VariableId,
) -> JsonValue {
    let var_is_true = graph.fix_network_variable(variable, true);
    let var_is_false = graph.fix_network_variable(variable, false);
    let all_colors = sinks.colors();
    let name = graph.as_network().get_variable_name(variable).clone();
    if sinks.intersect(&var_is_false).is_empty() {
        // Every sink is stable and has value true
        object! { "name": name, "constant": true }
    } else if sinks.intersect(&var_is_true).is_empty() {
        object! { "name": name, "constant": false }
    } else {
        // The variable value depends on attractor structure and parameters.
        let colors_where_true = sinks.intersect(&var_is_true).colors();
        let colors_where_false = sinks.intersect(&var_is_false).colors();
        let only_true_colors = all_colors.minus(&colors_where_false);
        let only_false_colors = all_colors.minus(&colors_where_true);
        let mixed_colors = colors_where_true.intersect(&colors_where_false);
        object! {
            "name": name,
            "only_true": only_true_colors.approx_cardinality(),
            "only_false": only_false_colors.approx_cardinality(),
            "mixed": mixed_colors.approx_cardinality(),
        }
    }
}
