use crate::scc::algo_async::{ReachAfterVarCanFire, GraphScheduler, FwdProcess, Process, FindExtendedComponent};
use biodivine_lib_param_bn::VariableId;
use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};

impl ReachAfterVarCanFire {
    pub fn name_tmp(variable: VariableId) -> String {
        format!("ReachAfterVarCanFire({})", variable)
    }

    pub fn mk(variable: VariableId, scheduler: &GraphScheduler, graph: &SymbolicAsyncGraph) -> ReachAfterVarCanFire {
        let var_can_fire = graph.var_can_post(variable, scheduler.get_universe());
        ReachAfterVarCanFire {
            name: Self::name_tmp(variable),
            variable,
            fwd: FwdProcess::mk(
                var_can_fire,
                scheduler.get_universe().clone(),
                scheduler.get_active_variables()
            )
        }
    }
}

impl Process for ReachAfterVarCanFire {
    fn step(&mut self, scheduler: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.fwd.step(scheduler, graph) {
            let var_can_fire = graph.var_can_post(self.variable, scheduler.get_universe());
            let reach_after_var_can_fire = self.fwd.get_reach_set();
            println!("Done reach after var can fire! {}", reach_after_var_can_fire.approx_cardinality());
            scheduler.spawn(Box::new(FindExtendedComponent::mk(var_can_fire, reach_after_var_can_fire.clone(), scheduler)));
            true
        } else {
            false
        }
    }

    fn weight(&self) -> usize {
        self.fwd.weight()
    }

    fn discard(&mut self, set: &GraphColoredVertices) {
        self.fwd.discard(set);
    }

    fn name(&self) -> &str {
        &self.name
    }
}