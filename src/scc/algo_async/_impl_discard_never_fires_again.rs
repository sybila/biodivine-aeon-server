use crate::scc::algo_async::{DiscardNeverFiresAgain, GraphScheduler, BwdProcess, Process, ReachAfterVarFired};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;

impl DiscardNeverFiresAgain {

    pub fn name_tmp(variable: VariableId) -> String {
        format!("DiscardNeverFiresAgain({})", variable)
    }

    pub fn mk(variable: VariableId, never_fires_again: GraphColoredVertices, scheduler: &GraphScheduler) -> DiscardNeverFiresAgain {
        DiscardNeverFiresAgain {
            name: Self::name_tmp(variable),
            variable,
            bwd: BwdProcess::mk(
                never_fires_again,
                scheduler.get_universe().clone(),
                scheduler.get_active_variables()
            ),
        }
    }

}

impl Process for DiscardNeverFiresAgain {

    fn step(&mut self, scheduler: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.bwd.step(scheduler, graph) {
            let basin = self.bwd.get_reach_set();
            scheduler.request_discard(basin.clone());
            scheduler.spawn(Box::new(ReachAfterVarFired::mk(self.variable, scheduler, graph)));
            for v in graph.network().variables() {
                if !scheduler.has(&DiscardNeverFiresAgain::name_tmp(v)) && !scheduler.has(&ReachAfterVarFired::name_tmp(v)) {
                    scheduler.spawn(Box::new(ReachAfterVarFired::mk(v, scheduler, graph)));
                }
            }
            true
        } else {
            false
        }
    }

    fn weight(&self) -> usize {
        self.bwd.weight()
    }

    fn discard(&mut self, set: &GraphColoredVertices) {
        self.bwd.discard(set);
    }

    fn name(&self) -> &str {
        &self.name
    }
}