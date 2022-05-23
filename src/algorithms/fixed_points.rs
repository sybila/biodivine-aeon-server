use crate::util::spawn_bound;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use biodivine_lib_param_bn::VariableId;
use futures::future::join_all;
use futures::FutureExt;
use std::sync::Arc;
use tokio::task::yield_now;

#[derive(Clone)]
pub struct FixedPoints<'a> {
    graph: &'a SymbolicAsyncGraph,
    candidates: GraphColoredVertices,
    transitions: Vec<VariableId>,
    parallel: bool,
}

impl<'a> FixedPoints<'a> {
    /// Create a `FixedPoints` instance with default algorithm parameters associated
    /// with the given `SymbolicAsyncGraph`.
    ///
    /// Default arguments:
    ///  - Tested `candidate` set is the whole set of vertices.
    ///  - Tested `transitions` are all network transitions.
    ///  - Runs in parallel if there are at least 4 cores and 30 symbolic variables.
    pub fn new(stg: &SymbolicAsyncGraph) -> FixedPoints {
        let cores = num_cpus::get_physical();
        let symbolic_variables = stg.symbolic_context().bdd_variable_set().num_vars();
        FixedPoints {
            graph: stg,
            candidates: stg.mk_unit_colored_vertices(),
            transitions: stg.as_network().variables().collect(),
            parallel: cores >= 4 && symbolic_variables > 30,
        }
    }

    /// Use parallelism when computing fixed-points.
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Use a specific set of candidate vertices from which fixed-points are taken.
    ///
    /// Note that this does not reduce the number of transitions. That is, a fixed-point
    /// within `candidates` must not have any outgoing transitions even outside of the
    /// `candidates` set.
    pub fn candidates(mut self, candidates: GraphColoredVertices) -> Self {
        self.candidates = candidates;
        self
    }

    /// Restrict the fixed-point search to a specific subset of network transitions.
    pub fn transitions(mut self, transitions: Vec<VariableId>) -> Self {
        self.transitions = transitions;
        self
    }

    /// Compute the fixed points using the parameters stored in this `FixedPoints` object.
    pub async fn compute(self) -> GraphColoredVertices {
        if self.parallel {
            Self::greedy_parallel(self.graph, &self.candidates, &self.transitions).await
        } else {
            Self::greedy(self.graph, &self.candidates, &self.transitions).await
        }
    }

    /// Naive algorithm for computing fixed-point attractors.
    ///
    /// The algorithm simply eliminates all states that can perform a transition one-by-one.
    /// The task can be cancelled after every non-trivial symbolic operation.
    pub async fn naive(
        stg: &SymbolicAsyncGraph,
        candidates: &GraphColoredVertices,
        transitions: &[VariableId],
    ) -> GraphColoredVertices {
        if cfg!(feature = "log-progress-fixed-points") {
            println!(
                "[FixedPoint](naive) Start algorithm with {} candidates and {} transitions.",
                candidates.approx_cardinality(),
                transitions.len()
            );
        }

        if transitions.is_empty() || candidates.is_empty() {
            if cfg!(feature = "log-progress-fixed-points") {
                println!("[FixedPoint](naive) Done. Nothing to compute.");
            }

            return candidates.clone();
        }

        let mut fixed_points = candidates.clone();

        for var in transitions {
            yield_now().await; // Potentially cancel here.
            let can_post = stg.var_can_post(*var, stg.unit_colored_vertices());
            fixed_points = fixed_points.minus(&can_post);
            if cfg!(feature = "log-progress-fixed-points") {
                println!(
                    "[FixedPoint](naive) Applied {:?}. Remaining candidates: [{} ~ {}].",
                    *var,
                    fixed_points.approx_cardinality(),
                    fixed_points.symbolic_size(),
                );
            }
        }

        if cfg!(feature = "log-progress-fixed-points") {
            println!(
                "[FixedPoint](naive) Algorithm done with {} fixed-points.",
                fixed_points.approx_cardinality()
            );
        }

        fixed_points
    }

    /// A greedy priority algorithm for computing fixed-point attractors.
    ///
    /// The algorithm also computes the intersection of all states without transition for each
    /// variable. However, it prioritises intersections of smaller symbolic sets. Consequently,
    /// the total number of BDD nodes tends to be smaller.
    pub async fn greedy(
        stg: &SymbolicAsyncGraph,
        candidates: &GraphColoredVertices,
        transitions: &[VariableId],
    ) -> GraphColoredVertices {
        if cfg!(feature = "log-progress-fixed-points") {
            println!(
                "[FixedPoint](greedy) Start algorithm with {} candidates and {} transitions.",
                candidates.approx_cardinality(),
                transitions.len()
            );
        }

        if transitions.is_empty() || candidates.is_empty() {
            if cfg!(feature = "log-progress-fixed-points") {
                println!("[FixedPoint](greedy) Done. Nothing to compute.");
            }

            return candidates.clone();
        }

        // Sets of vertices that *cannot* perform a particular transition.
        let mut candidates = transitions
            .iter()
            .map(|it| {
                let can_post = stg.var_can_post(*it, stg.unit_colored_vertices());
                stg.unit_colored_vertices().minus(&can_post)
            })
            .collect::<Vec<_>>();

        // To get all fixed-points, we need to compute the intersection of all of them.
        // But we can't just do it like in the naive algorithm, otherwise it might explode.

        while candidates.len() > 1 {
            // Put the smallest candidate set last.
            candidates.sort_by_cached_key(|it| it.symbolic_size());
            candidates.reverse();

            let item = candidates.pop().unwrap();

            for candidate in &mut candidates {
                yield_now().await; // Potentially cancel here.
                *candidate = candidate.intersect(&item);
            }

            if cfg!(feature = "log-progress-fixed-points") {
                let total_candidates = candidates
                    .iter()
                    .map(|it| it.approx_cardinality())
                    .fold(0.0, |a, b| a + b);
                let total_size = candidates
                    .iter()
                    .map(|it| it.symbolic_size())
                    .fold(0, |a, b| a + b);

                println!(
                    "[FixedPoint](greedy) Applied BDD with {} nodes. Remaining {} candidates, [{} ~ {}] in total.",
                    item.symbolic_size(),
                    candidates.len(),
                    total_candidates,
                    total_size
                );
            }
        }

        let fixed_points = candidates.into_iter().next().unwrap();

        if cfg!(feature = "log-progress-fixed-points") {
            println!(
                "[FixedPoint](greedy) Algorithm done with {} fixed-points.",
                fixed_points.approx_cardinality()
            );
        }

        fixed_points
    }

    /// Runs the same algorithm as `FixedPoints::greedy`, but tries to run individual symbolic
    /// operations as parallel `tokio` tasks.
    ///
    /// It does not scale super well since many realistic cases contain a single long-running
    /// task that cannot be parallelized, but it at least completes the other tasks in the
    /// remaining threads, so it still helps with the overall runtime.
    pub async fn greedy_parallel(
        stg: &SymbolicAsyncGraph,
        candidates: &GraphColoredVertices,
        transitions: &[VariableId],
    ) -> GraphColoredVertices {
        if cfg!(feature = "log-progress-fixed-points") {
            println!(
                "[FixedPoint](greedy,parallel) Start algorithm with {} candidates and {} transitions.",
                candidates.approx_cardinality(),
                transitions.len()
            );
        }

        if transitions.is_empty() || candidates.is_empty() {
            if cfg!(feature = "log-progress-fixed-points") {
                println!("[FixedPoint](greedy,parallel) Done. Nothing to compute.");
            }

            return candidates.clone();
        }

        // Sets of vertices that *cannot* perform a particular transition.

        let stg = Arc::new(stg.clone());
        let candidates = transitions
            .iter()
            .cloned()
            .map(|it| {
                let stg = stg.clone();
                spawn_bound(async move {
                    let can_post = stg.var_can_post(it, stg.unit_colored_vertices());
                    stg.unit_colored_vertices().minus(&can_post)
                })
                .map(|it| it.unwrap())
            })
            .collect::<Vec<_>>();

        let mut candidates = join_all(candidates.into_iter()).await;

        // To get all fixed-points, we need to compute the intersection of all of them.
        // But we can't just do it like in the naive algorithm, otherwise it might explode.

        while candidates.len() > 1 {
            // Put the smallest candidate set last.
            candidates.sort_by_cached_key(|it| it.symbolic_size());
            candidates.reverse();

            let item = Arc::new(candidates.pop().unwrap());

            let tasks = candidates
                .into_iter()
                .map(|candidate| {
                    let item = item.clone();
                    spawn_bound(async move { candidate.intersect(&item) }).map(|it| it.unwrap())
                })
                .collect::<Vec<_>>();

            candidates = join_all(tasks.into_iter()).await;

            if cfg!(feature = "log-progress-fixed-points") {
                let total_candidates = candidates
                    .iter()
                    .map(|it| it.approx_cardinality())
                    .fold(0.0, |a, b| a + b);
                let total_size = candidates
                    .iter()
                    .map(|it| it.symbolic_size())
                    .fold(0, |a, b| a + b);

                println!(
                    "[FixedPoint](greedy,parallel) Applied BDD with {} nodes. Remaining {} candidates, [{} ~ {}] in total.",
                    item.symbolic_size(),
                    candidates.len(),
                    total_candidates,
                    total_size
                );
            }
        }

        let fixed_points = candidates.into_iter().next().unwrap();

        if cfg!(feature = "log-progress-fixed-points") {
            println!(
                "[FixedPoint](greedy,parallel) Algorithm done with {} fixed-points.",
                fixed_points.approx_cardinality()
            );
        }

        fixed_points
    }
}
