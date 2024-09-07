use crate::algorithms::non_constant_variables;
use crate::algorithms::reachability::{bwd_step, fwd_step};
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;

/// A simplified "process" object that computes the forward reachable states from an initial set.
pub struct FwdProcess {
    stg: SymbolicAsyncGraph,
    set: GraphColoredVertices,
    variables: Vec<VariableId>,
}

/// A simplified "process" object that computes the backward reachable state from an initial set.
pub struct BwdProcess {
    stg: SymbolicAsyncGraph,
    set: GraphColoredVertices,
    variables: Vec<VariableId>,
}

impl FwdProcess {
    pub fn new(stg: SymbolicAsyncGraph, set: GraphColoredVertices) -> FwdProcess {
        FwdProcess {
            variables: non_constant_variables(&stg),
            stg,
            set,
        }
    }

    pub fn restrict(&mut self, universe: &GraphColoredVertices) {
        self.set = self.set.intersect(universe);
        self.stg = self.stg.restrict(universe);
        self.variables = non_constant_variables(&self.stg);
    }

    pub async fn step(&mut self) -> bool {
        if let Some(successors) = fwd_step(&self.stg, &self.set, &self.variables).await {
            self.set = self.set.union(&successors);
            false
        } else {
            true
        }
    }

    pub fn finish(&self) -> GraphColoredVertices {
        self.set.clone()
    }

    pub fn weight(&self) -> usize {
        self.set.symbolic_size()
    }
}

impl BwdProcess {
    pub fn new(stg: SymbolicAsyncGraph, set: GraphColoredVertices) -> BwdProcess {
        BwdProcess {
            variables: non_constant_variables(&stg),
            stg,
            set,
        }
    }

    pub fn restrict(&mut self, universe: &GraphColoredVertices) {
        self.set = self.set.intersect(universe);
        self.stg = self.stg.restrict(universe);
        self.variables = non_constant_variables(&self.stg);
    }

    pub async fn step(&mut self) -> bool {
        if let Some(successors) = bwd_step(&self.stg, &self.set, &self.variables).await {
            self.set = self.set.union(&successors);
            false
        } else {
            true
        }
    }

    pub fn finish(&self) -> GraphColoredVertices {
        self.set.clone()
    }

    pub fn weight(&self) -> usize {
        self.set.symbolic_size()
    }
}
