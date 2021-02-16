use crate::scc::algo_async::{BwdProcess, FwdProcess, GraphScheduler, Process};
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;

impl FwdProcess {
    pub fn mk(
        initial: GraphColoredVertices,
        in_universe: GraphColoredVertices,
        variables: &[VariableId],
    ) -> FwdProcess {
        if variables.is_empty() {
            panic!("Cannot compute reachability with no variables.");
        }
        FwdProcess {
            name: format!("Fwd({})", initial.approx_cardinality()),
            fwd_set: initial.intersect(&in_universe),
            universe: in_universe,
            active_variable: variables.len() - 1,
            variables: variables.to_vec(),
        }
    }

    pub fn get_reach_set(&self) -> &GraphColoredVertices {
        &self.fwd_set
    }
}

impl BwdProcess {
    pub fn mk(
        initial: GraphColoredVertices,
        in_universe: GraphColoredVertices,
        variables: &[VariableId],
    ) -> BwdProcess {
        if variables.is_empty() {
            panic!("Cannot compute reachability with no variables.");
        }
        BwdProcess {
            name: format!("Bwd({})", initial.approx_cardinality()),
            bwd_set: initial.intersect(&in_universe),
            universe: in_universe,
            active_variable: variables.len() - 1,
            variables: variables.to_vec(),
        }
    }

    pub fn get_reach_set(&self) -> &GraphColoredVertices {
        &self.bwd_set
    }
}

impl Process for FwdProcess {
    fn step(&mut self, _: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        let step_var = self.variables[self.active_variable];
        let post = graph
            .var_post(step_var, &self.fwd_set)
            .intersect(&self.universe)
            .minus(&self.fwd_set);

        if !post.is_empty() {
            self.fwd_set = self.fwd_set.union(&post);
            self.active_variable = self.variables.len() - 1;
            false
        } else {
            if self.active_variable != 0 {
                self.active_variable = self.active_variable - 1;
                false
            } else {
                true
            }
        }
    }

    fn weight(&self) -> usize {
        self.fwd_set.as_bdd().size()
    }

    fn discard(&mut self, set: &GraphColoredVertices) {
        self.fwd_set = self.fwd_set.minus(set);
        self.universe = self.universe.minus(set);
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Process for BwdProcess {
    fn step(&mut self, _: &mut GraphScheduler, graph: &SymbolicAsyncGraph) -> bool {
        let step_var = self.variables[self.active_variable];
        let pre = graph
            .var_pre(step_var, &self.bwd_set)
            .intersect(&self.universe)
            .minus(&self.bwd_set);

        if !pre.is_empty() {
            self.bwd_set = self.bwd_set.union(&pre);
            self.active_variable = self.variables.len() - 1;
            false
        } else {
            if self.active_variable != 0 {
                self.active_variable = self.active_variable - 1;
                false
            } else {
                true
            }
        }
    }

    fn weight(&self) -> usize {
        self.bwd_set.as_bdd().size()
    }

    fn discard(&mut self, set: &GraphColoredVertices) {
        self.bwd_set = self.bwd_set.minus(set);
        self.universe = self.universe.minus(set);
    }

    fn name(&self) -> &str {
        &self.name
    }
}
