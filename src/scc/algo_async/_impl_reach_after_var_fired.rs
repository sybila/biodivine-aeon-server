use crate::scc::algo_async::{
    DiscardNeverFiresAgain, FwdProcess, GraphScheduler, Process, ReachAfterVarFired,
};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;

impl ReachAfterVarFired {
    pub fn name_tmp(variable: VariableId) -> String {
        format!("ReachAfterVarFired({})", variable)
    }

    pub fn mk(
        variable: VariableId,
        scheduler: &GraphScheduler,
        graph: &SymbolicAsyncGraph,
    ) -> ReachAfterVarFired {
        let var_can_fire = graph.var_can_post(variable, scheduler.get_universe());
        let var_fired = graph.var_post(variable, &var_can_fire);
        ReachAfterVarFired {
            name: Self::name_tmp(variable),
            variable,
            fwd: FwdProcess::mk(
                var_fired,
                //var_can_fire,
                scheduler.get_universe().clone(),
                //graph.mk_unit_vertices(),
                scheduler.get_active_variables(),
            ),
        }
    }
}

impl Process for ReachAfterVarFired {
    fn step(&mut self, scheduler: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.fwd.step(scheduler, graph) {
            let var_can_fire = graph.var_can_post(self.variable, scheduler.get_universe());
            let reach_after_var_fired = self.fwd.get_reach_set();
            let never_fires_again = var_can_fire.minus(reach_after_var_fired);
            //let var_fired = graph.var_post(self.variable, &var_can_fire);
            //scheduler.spawn(Box::new(FindExtendedComponent::mk(var_fired, reach_after_var_fired.clone(), scheduler)));
            //scheduler.spawn(Box::new(FindExtendedComponent::mk(var_can_fire, reach_after_var_fired.clone(), scheduler)));
            if !never_fires_again.is_empty() {
                //scheduler.spawn(Box::new(DiscardNeverFiresAgain::mk(self.variable, never_fires_again, scheduler)));
                let mut process =
                    DiscardNeverFiresAgain::mk(self.variable, never_fires_again, scheduler);
                while !process.step(scheduler, graph) {}
            }
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
