use crate::scc::algo_interleaved_transition_guided_reduction::{Process, Scheduler};
use crate::GraphTaskContext;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;
use std::cmp::Ordering;

impl Scheduler<'_> {
    /// Create a new `Scheduler` with initial universe and active variables.
    pub fn new(
        ctx: &GraphTaskContext,
        initial: GraphColoredVertices,
        variables: Vec<VariableId>,
    ) -> Scheduler {
        Scheduler {
            active_variables: variables,
            universe: initial,
            processes: Vec::new(),
            to_discard: None,
            ctx,
        }
    }

    /// Finalize this scheduler, returning the current universe and active variables.
    pub fn finalize(self) -> (GraphColoredVertices, Vec<VariableId>) {
        (self.universe, self.active_variables)
    }

    /// Remove given `var` from the list of active variables.
    pub fn discard_variable(&mut self, var: VariableId) {
        self.active_variables
            .iter()
            .position(|v| *v == var)
            .into_iter()
            .for_each(|index| {
                self.active_variables.remove(index);
            });
    }

    /// Remove given `set` from the universe of this scheduler.
    pub fn discard_vertices(&mut self, set: &GraphColoredVertices) {
        self.universe = self.universe.minus(set);
        if let Some(to_discard) = self.to_discard.as_mut() {
            *to_discard = to_discard.union(set);
        } else {
            self.to_discard = Some(set.clone());
        }

        // Also update remaining set to indicate progress.
        self.get_context().update_remaining(&self.universe);
    }

    /// Add a new process into this scheduler.
    pub fn spawn<P: 'static + Process>(&mut self, process: P) {
        self.processes
            .push((process.symbolic_size(), Box::new(process)));
    }

    /// Get the current universe set of the scheduler.
    pub fn get_universe(&self) -> &GraphColoredVertices {
        &self.universe
    }

    /// Get the list of currently active variables.
    pub fn get_active_variables(&self) -> &[VariableId] {
        &self.active_variables
    }

    /// Get context of this task (meta state, manages cancellation or progress).
    pub fn get_context(&self) -> &GraphTaskContext {
        &self.ctx
    }

    /// True if all processes are finished.
    pub fn is_done(&self) -> bool {
        self.processes.is_empty()
    }

    /// If possible, perform one computational step for one of the processes.
    pub fn step(&mut self, graph: &SymbolicAsyncGraph) {
        if self.is_done() {
            return;
        }

        // First, apply to_discard in all processes:
        if let Some(to_discard) = self.to_discard.as_ref() {
            for (w, process) in self.processes.iter_mut() {
                process.discard_states(to_discard);
                *w = process.symbolic_size();
            }
            self.to_discard = None;
        }

        // Second, put the best process in the last place. However, try to do so in a
        // deterministic manner, because otherwise we get a weird discrepancy between
        // running times on different machines.
        self.processes.sort_by(|(s1, p1), (s2, p2)| {
            if *s1 == *s2 {
                // If the size are the same, compare cardinalities (prefer larger cardinality)
                let cmp = p1
                    .symbolic_cardinality()
                    .partial_cmp(&p2.symbolic_cardinality());

                match cmp {
                    Some(Ordering::Equal) | None => p1.unique_key().cmp(&p2.unique_key()),
                    Some(cmp) => cmp,
                }
            } else {
                // If they have different sizes, make sure they are sorted in descending order.
                s1.cmp(s2).reverse()
            }
        });
        self.processes.sort_by_key(|(w, _)| usize::MAX - (*w));

        // Perform one step in a process
        if let Some((current_weight, mut process)) = self.processes.pop() {
            println!(
                "Advanced process {} with weight {}",
                process.name(),
                current_weight
            );
            let is_done = process.step(self, graph);
            if !is_done {
                self.processes.push((process.symbolic_size(), process))
            }
        }
    }
}
