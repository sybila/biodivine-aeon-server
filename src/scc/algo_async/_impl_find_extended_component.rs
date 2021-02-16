use crate::scc::algo_async::{
    BwdProcess, DiscardBottomBasin, FindExtendedComponent, GraphScheduler, Process,
};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};

impl FindExtendedComponent {
    pub fn mk(
        pivots: GraphColoredVertices,
        fwd_set: GraphColoredVertices,
        scheduler: &GraphScheduler,
    ) -> FindExtendedComponent {
        FindExtendedComponent {
            name: format!("FindExtendedComponent({})", fwd_set.approx_cardinality()),
            fwd_set: fwd_set.clone(),
            bwd: BwdProcess::mk(pivots, fwd_set, scheduler.get_active_variables().clone()),
        }
    }
}

impl Process for FindExtendedComponent {
    fn step(&mut self, scheduler: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        if self.bwd.step(scheduler, graph) {
            let extended_component = self.bwd.get_reach_set();
            let bottom_region = self.fwd_set.minus(extended_component);
            println!("Bottom region is: {}", bottom_region.approx_cardinality());
            if !bottom_region.is_empty() {
                //scheduler.spawn(Box::new(DiscardBottomBasin::mk(bottom_region, scheduler)));
                let mut process = DiscardBottomBasin::mk(bottom_region, scheduler);
                while !process.step(scheduler, graph) {}
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
        self.fwd_set = self.fwd_set.minus(set);
        self.bwd.discard(set);
    }

    fn name(&self) -> &str {
        &self.name
    }
}
