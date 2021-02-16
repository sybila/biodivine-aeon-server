use crate::scc::algo_itgr::{ReachableProcess, FwdProcess, Process, Scheduler, BwdProcess, ExtendedComponentProcess};
use biodivine_lib_param_bn::VariableId;
use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};

impl ReachableProcess {
    pub fn new(
        var: VariableId,
        graph: &SymbolicAsyncGraph,
        universe: GraphColoredVertices,
    ) -> ReachableProcess {
        let var_can_post = graph.var_can_post(var, &universe);
        ReachableProcess {
            variable: var,
            fwd: FwdProcess::new(var_can_post, universe)
        }
    }
}

impl Process for ReachableProcess {
    fn step(&mut self, scheduler: &mut Scheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.fwd.step(scheduler, graph) {
            let fwd_set = &self.fwd.fwd;

            // If fwd set is not the whole universe, it probably has a basin.
            if fwd_set != scheduler.get_universe() {
                let mut bwd = BwdProcess::new(
                    fwd_set.clone(),
                    scheduler.get_universe().clone()
                );
                while !bwd.step(scheduler, graph) {
                    if scheduler.get_context().is_cancelled() {
                        break;
                    }
                }
                let basin_only = bwd.bwd.minus(fwd_set);
                if !basin_only.is_empty() {
                    scheduler.discard_vertices(&basin_only);
                }
            }

            scheduler.spawn(ExtendedComponentProcess::new(
                self.variable,
                fwd_set.clone(),
                scheduler.get_universe().clone(),
                graph
            ));
            true
        } else {
            false
        }
    }

    fn weight(&self) -> usize {
        self.fwd.weight()
    }

    fn discard_states(&mut self, set: &GraphColoredVertices) {
        self.fwd.discard_states(set)
    }
}