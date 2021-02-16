use crate::scc::algo_itgr::{Process, BwdProcess, Scheduler, FwdProcess};
use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};
use biodivine_lib_param_bn::VariableId;

impl BwdProcess {
    pub fn new(initial: GraphColoredVertices, universe: GraphColoredVertices) -> BwdProcess {
        BwdProcess {
            universe,
            bwd: initial
        }
    }
}

impl FwdProcess {
    pub fn new(initial: GraphColoredVertices, universe: GraphColoredVertices) -> FwdProcess {
        FwdProcess {
            universe,
            fwd: initial
        }
    }
}

fn reach<F>(
    variables: &[VariableId],
    set: &mut GraphColoredVertices,
    universe: &GraphColoredVertices,
    step: F,
) -> bool
    where
        F: Fn(VariableId, &GraphColoredVertices) -> GraphColoredVertices {
    if variables.is_empty() {
        return true;
    }
    for var in variables.iter().rev() {
        let stepped = step(*var, set)
            .minus(set)
            .intersect(universe);

        if !stepped.is_empty() {
            *set = set.union(&stepped);
            return false;
        }
    }
    true
}

impl Process for BwdProcess {
    fn step(&mut self, scheduler: &mut Scheduler, graph: &SymbolicAsyncGraph) -> bool {
        reach(scheduler.get_active_variables(),&mut self.bwd, &self.universe,
              |var, set| graph.var_pre(var, set)
        )
    }

    fn weight(&self) -> usize {
        self.bwd.symbolic_size()
    }

    fn discard_states(&mut self, set: &GraphColoredVertices) {
        self.universe = self.universe.minus(set);
        self.bwd = self.bwd.minus(set);
    }
}

impl Process for FwdProcess {
    fn step(&mut self, scheduler: &mut Scheduler, graph: &SymbolicAsyncGraph) -> bool {
        reach(scheduler.get_active_variables(), &mut self.fwd, &self.universe,
            |var, set| graph.var_post(var, set)
        )
    }

    fn weight(&self) -> usize {
        self.fwd.symbolic_size()
    }

    fn discard_states(&mut self, set: &GraphColoredVertices) {
        self.universe = self.universe.minus(set);
        self.fwd = self.fwd.minus(set);
    }
}