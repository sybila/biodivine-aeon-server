use crate::algorithms::asymptotic_behaviour::AsymptoticBehaviour::{
    Disorder, Oscillation, Stability,
};
use crate::util::functional::Functional;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{
    GraphColoredVertices, GraphColors, SymbolicAsyncGraph,
};
use biodivine_lib_param_bn::VariableId;
use rocket::route::BoxFuture;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Describes a possible asymptotic behaviour of a discrete system.
/// Intuitively:
///     - `Stability` is when all of the system variables reach a fixed state.
///     - `Oscillation` is when some system variables change predictably and the rest
///     has a fixed state.
///     - `Disorder` is when some system variables change in a non-deterministic, unpredictable
///     manner (others may be stable or predictable though).
///
/// Note that as `Oscillation`, we currently consider only *deterministic* oscillation
/// (i.e. cycles), but a non-deterministic oscillation could be considered as well in the future.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, fixed_map::Key)]
pub enum AsymptoticBehaviour {
    Stability,
    Oscillation,
    Disorder,
}

impl AsymptoticBehaviour {
    /// List all possible behaviours.
    ///
    /// If possible, use this instead of listing behaviours
    /// manually, because this will still work if we start adding new behaviours. Or at least
    /// break deterministically.
    pub fn behaviours() -> [AsymptoticBehaviour; 3] {
        [Stability, Oscillation, Disorder]
    }
}

/// A very simple fixed-size map for passing classification results around.
///
/// The main point of this structure is that it always has some (at least empty) value for
/// every type of asymptotic behaviour.
#[derive(Clone)]
pub struct AsymptoticBehaviourMap([GraphColors; 3]);

impl Debug for AsymptoticBehaviourMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AsymptoticBehaviourMap[stability={}, oscillation={}, disorder={}]",
            self.0[0].approx_cardinality(),
            self.0[1].approx_cardinality(),
            self.0[2].approx_cardinality(),
        )
    }
}

impl AsymptoticBehaviourMap {
    /// Create a new map initialised in the context of the given `SymbolicAsyncGraph`.
    fn new(stg: &SymbolicAsyncGraph) -> AsymptoticBehaviourMap {
        AsymptoticBehaviourMap([
            stg.mk_empty_colors(),
            stg.mk_empty_colors(),
            stg.mk_empty_colors(),
        ])
    }

    pub fn get(&self, behaviour: AsymptoticBehaviour) -> &GraphColors {
        match behaviour {
            Stability => &self.0[0],
            Oscillation => &self.0[1],
            Disorder => &self.0[2],
        }
    }

    pub fn export(self, behaviour: AsymptoticBehaviour) -> GraphColors {
        let array = self.0;
        match behaviour {
            Stability => array.into_iter().nth(0),
            Oscillation => array.into_iter().nth(1),
            Disorder => array.into_iter().nth(2),
        }
        .unwrap()
    }

    pub fn set(&mut self, behaviour: AsymptoticBehaviour, colors: GraphColors) {
        match behaviour {
            Stability => {
                self.0[0] = colors;
            }
            Oscillation => {
                self.0[1] = colors;
            }
            Disorder => {
                self.0[2] = colors;
            }
        }
    }

    /// Exports the non-empty values from this map into a vector.
    pub fn to_vec(self) -> Vec<(AsymptoticBehaviour, GraphColors)> {
        let [stability, oscillation, disorder] = self.0;
        Vec::new().apply(|it| {
            if !stability.is_empty() {
                it.push((Stability, stability));
            }
            if !oscillation.is_empty() {
                it.push((Oscillation, oscillation));
            }
            if !disorder.is_empty() {
                it.push((Disorder, disorder));
            }
        })
    }
}

/// Basic long-term behaviour classification algorithms.
impl AsymptoticBehaviour {
    /// Compute the subset of `set.colours()` for which the given `variable` does not change in
    /// *any* state of the given `set`.
    ///
    /// Formally, a `colour` is included in the result if no state in `set` can update the given
    /// `variable` in this `colour`.
    ///
    /// Note that it doesn't matter whether the variable update leaves the `set` or stays within.
    /// Both count as unstable. However, you can use `SymbolicAsyncGraph::restrict` to disregard
    /// updates leading outside of the provided `set`.
    pub fn check_variable_stability(
        stg: &SymbolicAsyncGraph,
        set: &GraphColoredVertices,
        variable: VariableId,
    ) -> GraphColors {
        let can_change = stg.var_can_post(variable, set);
        set.colors().minus(&can_change.colors())
    }

    /// Given a coloured `set` of states, identify which colours correspond to *stable*
    /// asymptotic behaviour.
    ///
    /// Formally, a colour has stable asymptotic behaviour if all states that are present in
    /// `set` for this colour have no outgoing transitions within the given `stg`. Note that
    /// there can be multiple stable states per colour as long as none has an outgoing transition.
    ///
    /// This method should be slightly faster than computing `AsymptoticBehaviour::classify`
    /// in full.
    pub fn check_stability(stg: &SymbolicAsyncGraph, set: &GraphColoredVertices) -> GraphColors {
        stg.as_network()
            .variables()
            .map(|var| Self::check_variable_stability(stg, set, var))
            .fold(set.colors(), |a, b| a.intersect(&b))
    }

    /// Given a coloured `set` of states, identify which colours correspond to *oscillating*
    /// asymptotic behaviour.
    ///
    /// Formally, a `colour` has oscillating asymptotic behaviour if all states that are present
    /// in `set` for this `colour` belong to deterministic cycles in `stg`. Note that
    /// there can be multiple cycles per `colour` in `set`, but they need to be completely
    /// disjoint.
    ///
    /// This method should be roughly equivalent in speed to computing
    /// `AsymptoticBehaviour::classify`.
    pub fn check_oscillation(stg: &SymbolicAsyncGraph, set: &GraphColoredVertices) -> GraphColors {
        Self::classify(stg, set).export(Oscillation)
    }

    /// Given a coloured `set` of states, identify which colours correspond to *disordered*
    /// asymptotic behaviour.
    ///
    /// Formally, a `colour` has a disordered behaviour if the set contains multiple intersecting
    /// cycles in this `colour`.
    ///
    /// This method should be roughly equivalent in speed to computing
    /// `AsymptoticBehaviour::classify`.
    pub fn check_disorder(stg: &SymbolicAsyncGraph, set: &GraphColoredVertices) -> GraphColors {
        Self::classify(stg, set).export(Disorder)
    }

    /// Classify the asymptotic behaviour within the given symbolic `set` as `stable`,
    /// `oscillating` or `disordered`. The result is a fixed-size map that is indexed by the
    /// three possible types of behaviour.
    ///
    /// Note that when the behaviour is not present in the `set`, the key
    /// is entirely omitted from the resulting map.
    ///
    /// Keep in mind  that you can also use `SymbolicAsyncGraph::restrict` to reduce the scope
    /// of the `stg` to limit the context of the classification to a smaller sub-graph.
    /// For example, given an arbitrary coloured SCC, you can restrict the `stg` to this SCC and
    /// then use `classify` to compute classification that disregards any outgoing/incoming
    /// transitions from this SCC.
    pub fn classify(
        stg: &SymbolicAsyncGraph,
        set: &GraphColoredVertices,
    ) -> AsymptoticBehaviourMap {
        // The idea is that we gradually move the vertices from one set to the other as new
        // transitions are discovered. Any color that appears in `successor_zero` or
        // `successor_more` is guaranteed to not have only deterministic cycles.
        let mut successors_zero = set.clone();
        let mut successors_one = stg.mk_empty_vertices();
        let mut successors_more = stg.mk_empty_vertices();

        for var in stg.as_network().variables() {
            let can_change = stg.var_can_post(var, set);

            let move_to_one = successors_zero.intersect(&can_change);
            let move_to_more = successors_one.intersect(&can_change);

            successors_zero = successors_zero.minus(&move_to_one);
            successors_one = successors_one.minus(&move_to_more).union(&move_to_one);
            successors_more = successors_more.union(&move_to_more);
        }

        let stability = successors_zero
            .colors()
            .minus(&successors_one.colors())
            .minus(&successors_more.colors());
        let oscillation = successors_one
            .colors()
            .minus(&successors_zero.colors())
            .minus(&successors_more.colors());
        let disorder = set.colors().minus(&stability).minus(&oscillation);

        AsymptoticBehaviourMap::new(stg).apply(|it| {
            it.set(Stability, stability);
            it.set(Oscillation, oscillation);
            it.set(Disorder, disorder);
        })
    }
}

/// Parallel long-term classification algorithms.
impl AsymptoticBehaviour {
    /// The same as `AsymptoticBehaviour::classify`, but uses `tokio` tasks
    /// to perform the classification in parallel for each variable.
    ///
    /// Since the task can outlive the function scope, arguments must be ref-counted.
    pub async fn classify_parallel(
        stg: Arc<SymbolicAsyncGraph>,
        set: Arc<GraphColoredVertices>,
    ) -> AsymptoticBehaviourMap {
        let all_colors = set.colors();
        let variables = stg.as_network().variables().collect();
        let [zero, one, more] = Self::classify_recursive(stg.clone(), set.clone(), variables).await;

        let stability = zero.colors().minus(&one.colors()).minus(&more.colors());
        let oscillation = one.colors().minus(&zero.colors()).minus(&more.colors());
        let disorder = all_colors.minus(&stability).minus(&oscillation);

        AsymptoticBehaviourMap::new(stg.as_ref()).apply(|it| {
            it.set(Stability, stability);
            it.set(Oscillation, oscillation);
            it.set(Disorder, disorder);
        })
    }

    fn classify_recursive(
        stg: Arc<SymbolicAsyncGraph>,
        set: Arc<GraphColoredVertices>,
        variables: Vec<VariableId>,
    ) -> BoxFuture<'static, [GraphColoredVertices; 3]> {
        return if variables.len() == 1 {
            // If there is only one variable remaining, compute states that can perform transition
            // with this variable. These are marked as "one transition", remaining are
            // "zero transitions".
            let var = variables[0];
            Box::pin(async move {
                let can_post = stg.var_can_post(var, &set);
                [set.minus(&can_post), can_post, stg.mk_empty_vertices()]
            })
        } else {
            // If there are more variables, split into two branches and continue each
            // in a new parallel task.
            let (left, right) = variables.split_at(variables.len() / 2);
            let (left, right) = (Vec::from(left), Vec::from(right));
            Box::pin(async move {
                let left = Self::classify_recursive(stg.clone(), set.clone(), left);
                let right = Self::classify_recursive(stg.clone(), set.clone(), right);
                let left = tokio::spawn(left);
                let right = tokio::spawn(right);
                let [l_zero, l_one, l_more] = left.await.unwrap();
                let [r_zero, r_one, r_more] = right.await.unwrap();

                let move_to_more = l_one.intersect(&r_one);
                let more = l_more.union(&r_more).union(&move_to_more);
                let one = l_one.union(&r_one).minus(&more);
                let zero = l_zero.intersect(&r_zero);
                [zero, one, more]
            })
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithms::asymptotic_behaviour::AsymptoticBehaviour;
    use crate::algorithms::asymptotic_behaviour::AsymptoticBehaviour::{
        Disorder, Oscillation, Stability,
    };
    use crate::{assert_symbolic_eq, assert_symbolic_ne};
    use biodivine_lib_param_bn::biodivine_std::bitvector::ArrayBitVector;
    use biodivine_lib_param_bn::biodivine_std::traits::Set;
    use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
    use biodivine_lib_param_bn::BooleanNetwork;
    use std::sync::Arc;

    /*
       Each test runs the parallel and normal versions, comparing results, plus comparing
       to the specialised sequential versions and some easy to prove axioms where appropriate.
    */

    #[tokio::test]
    async fn test_stability_check_1() {
        // Always bistable with [true, true] and [false, false] sinks.
        let bn = BooleanNetwork::try_from(
            "\
            a -> b
            b -> a
        ",
        )
        .unwrap();

        let stg = SymbolicAsyncGraph::new(bn).unwrap();
        let all_colors = stg.mk_unit_colors();

        let t_state = stg.vertex(&ArrayBitVector::from(vec![true, true]));
        let f_state = stg.vertex(&ArrayBitVector::from(vec![false, false]));

        let sinks = t_state.union(&f_state);
        let sink_colors = AsymptoticBehaviour::check_stability(&stg, &sinks);
        assert_symbolic_eq!(sink_colors.as_bdd(), all_colors.as_bdd());

        let classification = AsymptoticBehaviour::classify(&stg, &sinks);
        let classification_par =
            AsymptoticBehaviour::classify_parallel(Arc::new(stg), Arc::new(sinks)).await;

        for behaviour in AsymptoticBehaviour::behaviours() {
            assert_symbolic_eq!(
                classification.get(behaviour).as_bdd(),
                classification_par.get(behaviour).as_bdd()
            );
        }

        assert_symbolic_eq!(
            sink_colors.as_bdd(),
            classification.get(AsymptoticBehaviour::Stability).as_bdd()
        );
    }

    #[tokio::test]
    async fn test_stability_check_2() {
        // Is stable if `a` is not essential in `a`.
        let bn = BooleanNetwork::try_from(
            "\
            a -> b
            b -> a
            a -|? a
        ",
        )
        .unwrap();

        let stg = SymbolicAsyncGraph::new(bn).unwrap();
        let all_colors = stg.mk_unit_colors();

        let t_state = stg.vertex(&ArrayBitVector::from(vec![true, true]));
        let f_state = stg.vertex(&ArrayBitVector::from(vec![false, false]));

        let sinks = t_state.union(&f_state);
        let sink_colors = AsymptoticBehaviour::check_stability(&stg, &sinks);
        assert!(!sink_colors.is_empty());
        assert_symbolic_ne!(sink_colors.as_bdd(), all_colors.as_bdd());

        let classification = AsymptoticBehaviour::classify(&stg, &sinks);
        let classification_par =
            AsymptoticBehaviour::classify_parallel(Arc::new(stg), Arc::new(sinks)).await;

        for behaviour in AsymptoticBehaviour::behaviours() {
            assert_symbolic_eq!(
                classification.get(behaviour).as_bdd(),
                classification_par.get(behaviour).as_bdd()
            );
        }

        assert_symbolic_eq!(
            sink_colors.as_bdd(),
            classification.get(AsymptoticBehaviour::Stability).as_bdd()
        );
    }

    #[tokio::test]
    async fn test_oscillation_check() {
        // There is an oscillation on `b` if `a = true` and the update function of `b`
        // is chosen correctly.
        let bn = BooleanNetwork::try_from(
            "\
            a -> a
            a -> b
            b -| b
        ",
        )
        .unwrap();

        let stg = SymbolicAsyncGraph::new(bn).unwrap();
        let all_colors = stg.mk_unit_colors();

        let t_state = stg.vertex(&ArrayBitVector::from(vec![true, true]));
        let f_state = stg.vertex(&ArrayBitVector::from(vec![true, false]));
        let cycle = t_state.union(&f_state);

        let oscillation_colors = AsymptoticBehaviour::check_oscillation(&stg, &cycle);
        assert!(!oscillation_colors.is_empty());
        assert_symbolic_ne!(oscillation_colors.as_bdd(), all_colors.as_bdd());

        let classification = AsymptoticBehaviour::classify(&stg, &cycle);
        let classification_par =
            AsymptoticBehaviour::classify_parallel(Arc::new(stg), Arc::new(cycle)).await;

        for behaviour in AsymptoticBehaviour::behaviours() {
            assert_symbolic_eq!(
                classification.get(behaviour).as_bdd(),
                classification_par.get(behaviour).as_bdd()
            );
        }

        assert_symbolic_eq!(
            oscillation_colors.as_bdd(),
            classification
                .get(AsymptoticBehaviour::Oscillation)
                .as_bdd()
        );
    }

    #[tokio::test]
    async fn test_disorder() {
        // There is only one color leading to a disordered set.
        let bn = BooleanNetwork::try_from(
            "\
            a -| b
            b -| a
        ",
        )
        .unwrap();

        let stg = SymbolicAsyncGraph::new(bn).unwrap();
        let all_states = stg.mk_unit_colored_vertices();
        let all_colors = stg.mk_unit_colors();

        let disordered_colors = AsymptoticBehaviour::check_disorder(&stg, &all_states);
        assert_symbolic_eq!(disordered_colors.as_bdd(), all_colors.as_bdd());

        let classification = AsymptoticBehaviour::classify(&stg, &all_states);
        let classification_par =
            AsymptoticBehaviour::classify_parallel(Arc::new(stg), Arc::new(all_states)).await;

        for behaviour in AsymptoticBehaviour::behaviours() {
            assert_symbolic_eq!(
                classification.get(behaviour).as_bdd(),
                classification_par.get(behaviour).as_bdd()
            );
        }

        assert_symbolic_eq!(
            disordered_colors.as_bdd(),
            classification.get(AsymptoticBehaviour::Disorder).as_bdd()
        );
    }

    #[tokio::test]
    async fn test_classification() {
        // This BN admits literally everything. Four sinks, one/two cycles, or disorder.
        let bn = BooleanNetwork::try_from(
            "\
            a -?? a
            a -?? b
            b -?? b
            b -?? a
        ",
        )
        .unwrap();

        let stg = SymbolicAsyncGraph::new(bn).unwrap();
        let all_states = stg.mk_unit_colored_vertices();
        let all_colors = stg.mk_unit_colors();

        let classification = AsymptoticBehaviour::classify(&stg, &all_states);
        let classification_par =
            AsymptoticBehaviour::classify_parallel(Arc::new(stg), Arc::new(all_states)).await;

        for behaviour in AsymptoticBehaviour::behaviours() {
            assert_symbolic_eq!(
                classification.get(behaviour).as_bdd(),
                classification_par.get(behaviour).as_bdd()
            );
        }

        let stable = classification.get(Stability);
        let oscillating = classification.get(Oscillation);
        let disorder = classification.get(Disorder);

        assert!(!stable.is_empty());
        assert!(!oscillating.is_empty());
        assert!(!disorder.is_empty());
        assert_symbolic_ne!(stable.as_bdd(), all_colors.as_bdd());
        assert_symbolic_ne!(oscillating.as_bdd(), all_colors.as_bdd());
        assert_symbolic_ne!(disorder.as_bdd(), all_colors.as_bdd());
    }
}
