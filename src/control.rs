use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::{BooleanNetwork, VariableId, FnUpdate};
use std::collections::HashMap;
use std::convert::TryFrom;


fn reach_bwd_with_saturation(graph: &SymbolicAsyncGraph, initial: GraphColoredVertices) -> GraphColoredVertices {
    let mut result = initial;

    loop {
        let mut stop = true;
        for var in graph.as_network().variables().rev() { // The order is important to update Bdd based on the "easiest" variables first.
            let step = graph.var_pre(var, &result).minus(&result);

            if !step.is_empty() {
                result = result.union(&step);
                stop = false;
                break;
            }
        }
        if stop {
            return result;
        }
    }
}

fn reach_fwd_with_saturation(graph: &SymbolicAsyncGraph, initial: GraphColoredVertices) -> GraphColoredVertices {
    let mut result = initial;

    loop {
        let mut stop = true;
        for var in graph.as_network().variables().rev() { // The order is important to update Bdd based on the "easiest" variables first.
            let step = graph.var_post(var, &result).minus(&result);

            if !step.is_empty() {
                result = result.union(&step);
                stop = false;
                break;
            }
        }
        if stop {
            return result;
        }
    }
}


fn strong_basin(graph: &SymbolicAsyncGraph, initial: GraphColoredVertices) -> GraphColoredVertices {
    let mut basin = reach_bwd_with_saturation(graph, initial);

    loop {
        let mut stop = true;
        for var in graph.as_network().variables().rev() {
            let outside_successors = graph.var_post(var, &basin).minus(&basin);
            let can_go_out = graph.var_pre(var, &outside_successors).intersect(&basin);

            if !can_go_out.is_empty() {
                basin = basin.minus(&can_go_out);
                stop = false;
                break;
            }
        }
        if stop {
            return basin;
        }
    }
}

pub fn temporary_source_target_control(network: BooleanNetwork, source: &HashMap<VariableId, bool>, target: &HashMap<VariableId, bool>) {
    // First, make a controlled and uncontrolled asynchronous graphs. It is important to make sure
    // the networks have the same regulatory graphs and parameters. This ensures the networks
    // have the same symbolic representation.

    let mut controlled_network = BooleanNetwork::new(network.as_graph().clone());
    let mut uncontrolled_network = BooleanNetwork::new(network.as_graph().clone());

    // First, copy existing parameters from the original network:
    for p in network.parameters() {
        let parameter = network.get_parameter(p);
        let controlled_parameter = controlled_network.add_parameter(parameter.get_name(), parameter.get_arity()).unwrap();
        let uncontrolled_parameter = uncontrolled_network.add_parameter(parameter.get_name(), parameter.get_arity()).unwrap();
        if controlled_parameter != p || uncontrolled_parameter != p {
            panic!("This should not happen. Encodings should be the same.");
        }
    }

    // Then add control parameters and modify update functions:
    for v in network.variables().rev() {
        let v_controlled = format!("{}_controlled", network.get_variable_name(v));
        let controlled_parameter = controlled_network.add_parameter(v_controlled.as_str(), 0).unwrap();
        let uncontrolled_parameter = uncontrolled_network.add_parameter(v_controlled.as_str(), 0).unwrap();
        if controlled_parameter != uncontrolled_parameter {
            panic!("This should not happen. Encodings should be the same.");
        }
        let parameter_id = controlled_parameter;
        if let Some(function) = network.get_update_function(v) {
            // A little trick to avoid always cloning the value of fn_parameter...
            let fn_parameter = || FnUpdate::mk_param(parameter_id, &[]);

            // Set controlled function to (v_controlled => v) && (!v_controlled => f(...))
            let controlled_implies_v = fn_parameter().implies(FnUpdate::mk_var(v));
            let not_controlled_implies_f = FnUpdate::mk_not(fn_parameter()).implies(function.clone());
            let controlled_function = controlled_implies_v.and(not_controlled_implies_f);
            controlled_network.add_update_function(v, controlled_function).unwrap();

            // Set uncontrolled function to (v_controlled || !v_controlled) && f(...)
            let control_tautology = fn_parameter().or(FnUpdate::mk_not(fn_parameter()));
            let uncontrolled_function = control_tautology.and(function.clone());
            uncontrolled_network.add_update_function(v, uncontrolled_function).unwrap();
        } else {
            panic!(r"
                We assume there are no implicit unknown functions in the network.
                If there are, replace them with explicit parameters.
            ");
        }
    }

    let controlled_graph = SymbolicAsyncGraph::new(controlled_network).unwrap();
    let uncontrolled_graph = SymbolicAsyncGraph::new(uncontrolled_network).unwrap();

    // Check that the encoding is actually equivalent.
    for bdd_variable in controlled_graph.symbolic_context().bdd_variable_set().variables() {
        let controlled_name = controlled_graph.symbolic_context().bdd_variable_set().name_of(bdd_variable);
        let uncontrolled_name = uncontrolled_graph.symbolic_context().bdd_variable_set().name_of(bdd_variable);
        if controlled_name != uncontrolled_name {
            panic!(r"For some reasons, the encodings are not equivalent.");
        }
    }

    // Now, we can also make some helper sets that represent when variables are controlled:
    let mut var_is_controlled = HashMap::new();
    let mut var_is_not_controlled = HashMap::new();
    for v in network.variables() {
        let v_name = controlled_graph.as_network().get_variable_name(v);
        let parameter = controlled_graph.as_network().find_parameter(format!("{}_controlled", v_name).as_str()).unwrap();
        let bdd_v_is_controlled = controlled_graph.symbolic_context().mk_uninterpreted_function_is_true(parameter, &[]);
        let bdd_v_is_not_controlled = bdd_v_is_controlled.not();
        // This "copy" method is a workaround that "internally" creates a symbolic set with an arbitrary Bdd.
        // However, we are responsible for the Bdd being a valid symbolic representation of what we want.
        // It takes the metadata from the `unit_colors` symbolic set, but swaps the Bdd with our value.
        var_is_controlled.insert(v, controlled_graph.unit_colors().copy(bdd_v_is_controlled));
        var_is_not_controlled.insert(v, controlled_graph.unit_colors().copy(bdd_v_is_not_controlled));
    }

    // Now, we are in a very special position. Symbolic sets (i.e. `GraphColoredVertices`,
    // `GraphColors`, `GraphVertices`) and in fact all `Bdd` objects are interchangeable between
    // controlled and uncontrolled graph. We don't have to perform any conversion between them.
    // However: DO NOT DO THIS IN ANY OTHER SITUATION! If the encodings do not match exactly,
    // the behaviour is undefined.

    // First, let's restrict ourselves only to colors where source and target are both members
    // of attractors.

    let target_set = target.iter().fold(uncontrolled_graph.mk_unit_colored_vertices(), |a, (v, value)| {
        a.intersect(&uncontrolled_graph.fix_network_variable(*v, *value))
    });

    let source_set = source.iter().fold(uncontrolled_graph.mk_unit_colored_vertices(), |a, (v, value)| {
        a.intersect(&uncontrolled_graph.fix_network_variable(*v, *value))
    });

    let target_fwd = reach_fwd_with_saturation(&uncontrolled_graph, target_set.clone());
    let target_bwd = reach_bwd_with_saturation(&uncontrolled_graph, target_set.clone());
    let target_not_attractor = target_fwd.minus(&target_bwd).colors();

    let source_fwd = reach_fwd_with_saturation(&uncontrolled_graph, source_set.clone());
    let source_bwd = reach_bwd_with_saturation(&uncontrolled_graph, source_set.clone());
    let source_not_attractor = source_fwd.minus(&source_bwd).colors();

    // We have to also remove these colors, because these appear in situations where source and
    // target are in the same attractor.
    let target_can_reach_source = target_fwd.intersect(&source_fwd).colors();
    let both_are_attractors = uncontrolled_graph.unit_colors()
        .minus(&source_not_attractor)
        .minus(&target_not_attractor)
        .minus(&target_can_reach_source);

    // To get the actual number of colors, we have to disregard the new parameters (v_controlled)
    // which we have added and are completely unconstrained in this scenario.
    let state_variable_count = u16::try_from(network.num_vars()).unwrap();  // for each variable, we have added a parameter.
    let extra_count = (2.0f64).powi(state_variable_count.into());
    let actual_colors = both_are_attractors.approx_cardinality() / extra_count;
    println!("Both source and target appear in (different) attractors for {} colors.", actual_colors);

    // And make a new target set but only with these valid colors:
    let target_set = target_set.intersect_colors(&both_are_attractors);

    // This is a set of `(state, parametrisation)` such that all paths from `state` will
    // eventually always converge to `target` (under fairness) in the UNCONTROLLED graph.
    let basin_of_target = strong_basin(&uncontrolled_graph, target_set);

    // However, in the controlled graph, this is also the set `(state, control, parametrisation)`
    // such that if `control` is *released* on any path starting in `state`, `target` is
    // eventually reached.

    // Consequently, we can now compute a strong basin of the set in the CONTROLLED graph,
    // which will give us the state that will eventually always reach this basin of "good" states.
    let mut controlled_basin = strong_basin(&controlled_graph, basin_of_target);

    // This gives us triples `(state, control, parametrisation)` such that on all paths starting
    // in `state`, there is a point after which if `control` is released, the network will converge
    // to `target`.

    // Now what we need to do is to only keep states that are equivalent to source under control.
    // This boils down to removing any state where uncontrolled variables do not match the value
    // in `source` (since control application will fix all controlled variables).

    for v in network.variables() {
        let source_value = source.get(&v).unwrap().clone();
        let v_different_than_source = uncontrolled_graph.fix_network_variable(v, !source_value);
        let v_not_controlled = var_is_not_controlled.get(&v).unwrap();

        let v_not_controlled_and_different_than_source = v_different_than_source.intersect_colors(v_not_controlled);
        controlled_basin = controlled_basin.minus(&v_not_controlled_and_different_than_source);
    }

    // Now, we have our control mapping stored in the controlled basin. Specifically, in `v_controlled`,
    // we are keeping the information whether variable is controlled, and in the states themselves,
    // we are keeping the value of control. That is, to get the actual control vector for a
    // particular triple `(state, control, parametrisation)`, we need to look into `control` and see
    // which variables are controlled. Then, we need to look at their values in `state` to see
    // to what value they are controlled.
    //
    // That is not exactly practical, but we can just export the Bdd and manipulate that if we
    // wanted to perform some more advanced analysis. Alternatively, we can do something like this
    // to get some basic info from the mapping:

    let control_map = controlled_basin;

    for v in network.variables() {
        let variables_other_then_v_are_not_controlled = var_is_not_controlled.iter()
            .filter(|(id, _)| **id != v)
            .fold(uncontrolled_graph.mk_unit_colors(), |a, (_, b)| a.intersect(b));
        let only_v_is_controlled = var_is_controlled.get(&v).unwrap().intersect(&variables_other_then_v_are_not_controlled);

        // In this symbolic set, all values of `v_controlled` parameters are fixed. We could do
        // this more easily if we saved `BddVariables` that the `v_controlled` parameters correspond
        // to, but for now this is ok.
        let controls_where_only_v_is_controlled = control_map.intersect_colors(&only_v_is_controlled);

        let v_true = uncontrolled_graph.fix_network_variable(v, true);
        let v_false = uncontrolled_graph.fix_network_variable(v, false);

        println!("Control [{}=true] works for {} colors.", network.get_variable_name(v), v_true.intersect(&controls_where_only_v_is_controlled).colors().approx_cardinality());
        println!("Control [{}=false] works for {} colors.", network.get_variable_name(v), v_false.intersect(&controls_where_only_v_is_controlled).colors().approx_cardinality());
    }

}

#[cfg(test)]
mod tests {
    use biodivine_lib_param_bn::BooleanNetwork;
    use std::convert::TryFrom;
    use crate::control::temporary_source_target_control;
    use std::collections::HashMap;

    #[test]
    #[allow(non_snake_case)]
    fn test_g2a_control() {
        // This is the G2A network which is often used as an example, but here, we have added
        // an a self-loop to all variables that otherwise don't have one (because in a controlled
        // graph, technically every variable has a self-loop).

        // We have also given each variable an explicit update function.
        let g2a_network = r"
            $CtrA:f_5(CtrA, GcrA, CcrM, SciP)
            CtrA -> CtrA
            GcrA -> CtrA
            CcrM -| CtrA
            SciP -| CtrA
            $GcrA:f_4(CtrA, DnaA)
            GcrA -?? GcrA
            CtrA -| GcrA
            DnaA -> GcrA
            $CcrM:f_3(CtrA, CcrM, SciP)
            CtrA -> CcrM
            CcrM -| CcrM
            SciP -| CcrM
            $SciP:f_1(CtrA, DnaA)
            CtrA -> SciP
            DnaA -| SciP
            SciP -?? SciP
            $DnaA:f_2(CtrA, GcrA, DnaA, CcrM)
            CtrA -> DnaA
            GcrA -| DnaA
            DnaA -| DnaA
            CcrM -> DnaA
        ";

        let network = BooleanNetwork::try_from(g2a_network).unwrap();
        let CcrM = network.as_graph().find_variable("CcrM").unwrap();
        let CtrA = network.as_graph().find_variable("CtrA").unwrap();
        let DnaA = network.as_graph().find_variable("DnaA").unwrap();
        let GcrA = network.as_graph().find_variable("GcrA").unwrap();
        let SciP = network.as_graph().find_variable("SciP").unwrap();

        let mut source = HashMap::new();
        source.insert(CcrM, true);
        source.insert(CtrA, true);
        source.insert(DnaA, true);
        source.insert(GcrA, false);
        source.insert(SciP, false);

        let mut target = HashMap::new();
        target.insert(CcrM, true);
        target.insert(CtrA, false);
        target.insert(DnaA, false);
        target.insert(GcrA, false);
        target.insert(SciP, false);

        temporary_source_target_control(network, &source, &target);
    }

}