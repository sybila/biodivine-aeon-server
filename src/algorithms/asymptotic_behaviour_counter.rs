use crate::algorithms::asymptotic_behaviour::AsymptoticBehaviour::{
    Disorder, Oscillation, Stability,
};
use crate::algorithms::asymptotic_behaviour::{AsymptoticBehaviour, AsymptoticBehaviourMap};
use crate::algorithms::incremental_classifier::{Feature, IncrementalClassifier};
use crate::util::functional::Functional;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColors, SymbolicAsyncGraph};
use fixed_map::Map;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};

/// Used to distinguish different types of behaviour in the `AsymptoticBehaviourCounter`.
#[derive(Clone, Eq, PartialEq, Default)]
pub struct Count(Map<AsymptoticBehaviour, usize>);

impl Feature for Count {
    fn extend(&self, other: &Self) -> Self {
        let mut map = self.0;
        for (key, value) in other.0.iter() {
            if let Some(current) = map.get(key) {
                let new_count = *value + *current;
                map.insert(key, new_count);
            } else {
                map.insert(key, *value);
            }
        }
        Count(map)
    }
}

impl From<AsymptoticBehaviour> for Count {
    fn from(behaviour: AsymptoticBehaviour) -> Self {
        Count(Map::new().apply(|it| it.insert(behaviour, 1)))
    }
}

impl PartialOrd<Self> for Count {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Count {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Debug for Count {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BehaviourCount[stability={}, oscillation={}, disorder={}]",
            self.get(Stability),
            self.get(Oscillation),
            self.get(Disorder)
        )
    }
}

impl Count {
    pub fn contains(&self, behaviour: AsymptoticBehaviour) -> bool {
        self.0.get(behaviour).cloned().unwrap_or(0) > 0
    }

    pub fn get(&self, behaviour: AsymptoticBehaviour) -> usize {
        self.0.get(behaviour).cloned().unwrap_or(0)
    }

    pub fn set(&mut self, behaviour: AsymptoticBehaviour, value: usize) {
        self.0.insert(behaviour, value);
    }

    /// Produce the key used for sorting individual classes.
    ///
    /// For now, we assume that `Disorder > Oscillation > Stability` to break the ties
    /// in the lattice of possible sets.
    fn sort_key(&self) -> [usize; 3] {
        [
            self.get(Disorder),
            self.get(Oscillation),
            self.get(Stability),
        ]
    }
}

/// Implements a symbolic incremental classifier that tracks how many times have different types
/// of asymptotic behaviour been encountered.
pub struct AsymptoticBehaviourCounter {
    counter: IncrementalClassifier<Count, GraphColors>,
}

impl AsymptoticBehaviourCounter {
    /// Build a new classifier using the given unit color set.
    pub fn new(stg: &SymbolicAsyncGraph) -> AsymptoticBehaviourCounter {
        AsymptoticBehaviourCounter {
            counter: IncrementalClassifier::empty(stg.mk_unit_colors()),
        }
    }

    /// Number of classes encountered so far.
    pub fn len(&self) -> usize {
        self.counter.len()
    }

    pub fn is_empty(&self) -> bool {
        self.counter.is_empty()
    }

    /// Extend this classifier using a full behaviour classification map.
    pub fn add_classification(&mut self, classification: &AsymptoticBehaviourMap) {
        for (behaviour, colors) in classification.clone().to_vec() {
            self.counter.extend(&Count::from(behaviour), &colors);
        }
    }

    /// Extend this classifier with a single instance of observed behaviour.
    pub fn add(&mut self, behaviour: AsymptoticBehaviour, colors: &GraphColors) {
        self.counter.extend(&Count::from(behaviour), colors);
    }

    /// Get the full classification produced so far.
    pub fn classes(&self) -> &[(Count, GraphColors)] {
        self.counter.get_features()
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithms::asymptotic_behaviour::AsymptoticBehaviour;
    use crate::algorithms::AsymptoticBehaviourCounter;
    use crate::assert_symbolic_eq;
    use biodivine_lib_param_bn::biodivine_std::traits::Set;
    use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
    use biodivine_lib_param_bn::BooleanNetwork;

    #[test]
    fn test_asymptotic_behaviour_counter() {
        let bn = BooleanNetwork::try_from(
            r#"
            a -?? a
            a -?? b
            b -?? a
            b -?? b
        "#,
        )
        .unwrap();

        // This doesn't have a particular semantic meaning, it just checks that the classifier
        // works as expected.

        let a = bn.as_graph().find_variable("a").unwrap();
        let stg = SymbolicAsyncGraph::new(&bn).unwrap();

        let a_true = stg.fix_network_variable(a, true);
        let a_false = stg.fix_network_variable(a, false);

        let cls_a_true = AsymptoticBehaviour::classify(&stg, &a_true);
        let cls_a_false = AsymptoticBehaviour::classify(&stg, &a_false);

        let mut classifier = AsymptoticBehaviourCounter::new(&stg);
        assert_eq!(classifier.len(), 1);

        classifier.add_classification(&cls_a_true);
        assert_eq!(classifier.len(), 3);

        for (a, b) in cls_a_false.to_vec() {
            classifier.add(a, &b);
        }

        assert_eq!(classifier.len(), 6);

        // Pairwise disjoint:
        let classes = classifier.classes().iter().collect::<Vec<_>>();
        let mut all = stg.mk_empty_colors();
        for (a1, b1) in &classes {
            for (a2, b2) in &classes {
                if a1 != a2 {
                    assert!(b1.intersect(b2).is_empty());
                }
            }
            all = all.union(b1);
        }

        assert_symbolic_eq!(all.as_bdd(), stg.unit_colors().as_bdd());

        classifier.add_classification(&cls_a_true);
        // As opposed to classifier, here the number of combinations grows.
        assert_eq!(classifier.len(), 9);
    }
}
