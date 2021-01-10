use crate::scc::algo_async::{GraphScheduler, Process};
use biodivine_lib_param_bn::symbolic_async_graph::{SymbolicAsyncGraph, GraphColoredVertices};
use biodivine_lib_param_bn::VariableId;

/// Basic utility methods
impl GraphScheduler {

    /// Make a new scheduler initialized with the given universe and graph.
    pub fn mk(graph: &SymbolicAsyncGraph, universe: &GraphColoredVertices) -> GraphScheduler {
        GraphScheduler {
            universe: universe.clone(),
            active_variables: graph.network().variables().collect(),
            active_processes: vec![],
            discard_next: None
        }
    }

    pub fn get_universe(&self) -> &GraphColoredVertices {
        &self.universe
    }

    pub fn get_active_variables(&self) -> &[VariableId] {
        &self.active_variables
    }

    /// "Close" this scheduler and return the remaining universe and active variables.
    pub fn finalize(self) -> (GraphColoredVertices, Vec<VariableId>) {
        (self.universe, self.active_variables)
    }

}

/// Methods used to schedule the processes.
impl GraphScheduler {

    pub fn has(&self, process: &str) -> bool {
        self.active_processes.iter().any(|p| p.name() == process)
    }

    /// Enqueue a process into the scheduler.
    pub fn spawn(&mut self, process: Box<dyn Process>) {
        self.active_processes.push(process);
    }

    /// Mark the given set as discarded - it will be removed from all processes before the next step.
    pub fn request_discard(&mut self, set: GraphColoredVertices) {
        self.universe = self.universe.minus(&set);
        println!("New universe size: {}, nodes: {}", self.universe.approx_cardinality(), self.universe.as_bdd().size());
        if let Some(discard) = self.discard_next.as_mut() {
            *discard = discard.union(&set);
        } else {
            self.discard_next = Some(set);
        }
    }

    /// Mark the given variable as effectively constant - it will not be used in new reachability procedures.
    pub fn _request_variable_discard(&mut self, variable: VariableId) {
        self.active_variables = self.active_variables.iter().cloned().filter(|v| *v != variable).collect();
    }

    /// Pick a suitable process and perform one step in it.
    pub fn step(&mut self, graph: &SymbolicAsyncGraph) -> bool {

        // First, discard state space:
        if let Some(to_remove) = &self.discard_next {
            for process in self.active_processes.iter_mut() {
                process.discard(to_remove);
            }
            self.discard_next = None;
        }

        // Second, find process with smallest weight
        let (pid, _) = self.active_processes.iter().enumerate()
            .min_by_key(|(_, p)| p.weight()).unwrap();
        //let pid = self.active_processes.len() - 1;    // simplified process selection for debugging

        // Workaround to drop immutable borrow to self.
        let mut process = self.active_processes.remove(pid);

        // Now do one step in the process
        /*let mut k = 0;
        for i in 0..10 {
            if process.step(self, graph) { break; }
            k = i;
        }
        if k == 9 {
            self.active_processes.push(process);
        }*/
        if !process.step(self, graph) {
            // Only put it back if it is not done.
            self.active_processes.push(process);
        } else {
            println!("Process finished, new universe size is {}. {} processes, {} variables remaining.",
                     self.universe.approx_cardinality(),
                     self.active_processes.len(),
                     self.active_variables.len()
            );
        }

        // We are done when there is nothing else to compute.
        self.active_processes.is_empty()
    }

}