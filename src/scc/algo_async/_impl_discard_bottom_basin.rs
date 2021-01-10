use crate::scc::algo_async::{DiscardBottomBasin, GraphScheduler, BwdProcess, Process};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};

impl DiscardBottomBasin {
    pub fn mk(bottom_region: GraphColoredVertices, scheduler: &GraphScheduler) -> DiscardBottomBasin {
        DiscardBottomBasin {
            name: format!("DiscardBottomBasin({})", bottom_region.approx_cardinality()),
            bottom_region: bottom_region.clone(),
            bwd: BwdProcess::mk(
                bottom_region,
                scheduler.get_universe().clone(),
                scheduler.get_active_variables()
            ),
        }
    }
}

impl Process for DiscardBottomBasin {

    fn step(&mut self, scheduler: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.bwd.step(scheduler, graph) {
            let bottom_basin = self.bwd.get_reach_set();
            let only_basin = bottom_basin.minus(&self.bottom_region);
            scheduler.request_discard(only_basin);
            true
        } else {
            false
        }
    }

    fn weight(&self) -> usize {
        self.bwd.weight()
    }

    fn discard(&mut self, set: &GraphColoredVertices) {
        self.bottom_region = self.bottom_region.minus(set);
        self.bwd.discard(set);
    }

    fn name(&self) -> &str {
        &self.name
    }
}