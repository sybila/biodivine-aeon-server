use crate::scc::algo_stability_analysis::{AttractorStabilityData, Stability};
use crate::util::functional::Functional;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{
    GraphColoredVertices, GraphColors, SymbolicAsyncGraph,
};
use biodivine_lib_param_bn::VariableId;
use std::ops::Index;

impl Index<Stability> for AttractorStabilityData {
    type Output = GraphColors;

    fn index(&self, index: Stability) -> &Self::Output {
        match index {
            Stability::True => &self.stability_true,
            Stability::False => &self.stability_false,
            Stability::Unstable => &self.unstable,
        }
    }
}

impl AttractorStabilityData {
    /// Perform stability analysis for one attractor and one variable.
    pub fn for_attractor(
        graph: &SymbolicAsyncGraph,
        attractor: &GraphColoredVertices,
        variable: VariableId,
    ) -> AttractorStabilityData {
        let var_is_true = graph.fix_network_variable(variable, true);
        let var_is_false = graph.fix_network_variable(variable, false);
        let colors_with_true = attractor.intersect(&var_is_true).colors();
        let colors_with_false = attractor.intersect(&var_is_false).colors();
        let colors_with_both = colors_with_true.intersect(&colors_with_false);
        AttractorStabilityData {
            stability_true: colors_with_true.minus(&colors_with_both),
            stability_false: colors_with_false.minus(&colors_with_both),
            unstable: colors_with_both,
        }
        .also(|data| {
            let all = data
                .stability_true
                .union(&data.stability_false)
                .union(&data.unstable);
            if all != attractor.colors() {
                panic!("Mismatched attractor colors.");
            }
            if !data
                .stability_true
                .intersect(&data.stability_false)
                .is_empty()
            {
                panic!("FAIL");
            }
            if !data.stability_false.intersect(&data.unstable).is_empty() {
                panic!("FAIL");
            }
            if !data.unstable.intersect(&data.stability_true).is_empty() {
                panic!("FAIL");
            }
        })
    }
}
