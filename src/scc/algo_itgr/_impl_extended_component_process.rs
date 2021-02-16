use crate::scc::algo_itgr::{ExtendedComponentProcess, BwdProcess, Process, Scheduler};
use biodivine_lib_param_bn::VariableId;
use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};
use std::sync::atomic::Ordering;

impl ExtendedComponentProcess {
    pub fn new(
        variable: VariableId,
        fwd_set: GraphColoredVertices,
        universe: GraphColoredVertices,
        graph: &SymbolicAsyncGraph,
    ) -> ExtendedComponentProcess {
        let var_can_post = graph.var_can_post(variable, &universe);
        ExtendedComponentProcess {
            variable,
            fwd_set: fwd_set.clone(),
            bwd: BwdProcess::new(var_can_post, fwd_set),
        }
    }
}

impl Process for ExtendedComponentProcess {
    fn step(&mut self, scheduler: &mut Scheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.bwd.step(scheduler, graph) {
            let extended_component = &self.bwd.bwd;
            let bottom = self.fwd_set.minus(extended_component);

            if !bottom.is_empty() {
                let mut bwd = BwdProcess::new(
                    bottom.clone(),
                    scheduler.get_universe().clone(),
                );
                while !bwd.step(scheduler, graph) {
                    if scheduler.get_context().is_cancelled() {
                        break;
                    }
                }
                let basin_only = bwd.bwd.minus(&bottom);
                if !basin_only.is_empty() {
                    scheduler.discard_vertices(&basin_only);
                }
            }

            let var_can_post = graph.var_can_post(self.variable, scheduler.get_universe());
            if var_can_post.is_empty() {
                scheduler.discard_variable(self.variable);
            }

            true
        } else {
            false
        }
    }

    fn weight(&self) -> usize {
        self.bwd.weight()
    }

    fn discard_states(&mut self, set: &GraphColoredVertices) {
        self.bwd.discard_states(set);
        self.fwd_set = self.fwd_set.minus(set);
    }
}