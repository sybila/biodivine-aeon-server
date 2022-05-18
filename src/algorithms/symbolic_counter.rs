use crate::algorithms::incremental_classifier::{Feature, IncrementalClassifier};
use crate::util::functional::Functional;
use biodivine_lib_param_bn::biodivine_std::traits::Set;

/// Basic `SymbolicCounter` object that uses `IncrementalClassifier` to count how many times
/// a a particular set member has been observed.
#[derive(Clone)]
pub struct SymbolicCounter<S: Set> {
    counter: IncrementalClassifier<Count, S>,
}

/// Just a private wrapper around `usize` that implements `Feature`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Count(usize);

impl Feature for Count {
    fn extend(&self, other: &Self) -> Self {
        Count(self.0 + other.0)
    }
}

impl<S: Set> SymbolicCounter<S> {
    /// Create a new counter initialized to zero.
    pub fn new(unit_set: S) -> SymbolicCounter<S> {
        SymbolicCounter {
            counter: IncrementalClassifier::empty(unit_set),
        }
    }

    /// Increment the members of the given `set`.
    pub fn increment(&mut self, set: &S) {
        self.counter.extend(&Count(1), set);
    }

    /// Return the elements with the maximal count so far.
    pub fn max_count(&self) -> (usize, &S) {
        self.counter
            .get_features()
            .iter()
            .max_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .expect("Unreachable: empty symbolic counter.")
            .and_then(|(k, v)| (k.0, v))
    }

    /// Return the elements with the minimal count so far.
    pub fn min_count(&self) -> (usize, &S) {
        self.counter
            .get_features()
            .iter()
            .min_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .expect("Unreachable: empty symbolic counter.")
            .and_then(|(k, v)| (k.0, v))
    }

    /// Take a snapshot of the values that are currently stored in the counter.
    pub fn export_counts(&self) -> Vec<(usize, S)> {
        self.counter
            .get_features()
            .iter()
            .map(|(k, v)| (k.0, v.clone()))
            .collect::<Vec<_>>()
            .apply(|it| it.sort_by_key(|(k, _)| *k))
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithms::symbolic_counter::SymbolicCounter;
    use biodivine_lib_param_bn::biodivine_std::traits::Set;
    use std::collections::HashSet;

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestSet(HashSet<usize>);

    impl TestSet {
        pub fn new(values: &[usize]) -> TestSet {
            TestSet(values.iter().cloned().collect())
        }
    }

    impl Set for TestSet {
        fn union(&self, other: &Self) -> Self {
            Self(self.0.union(&other.0).cloned().collect())
        }

        fn intersect(&self, other: &Self) -> Self {
            Self(self.0.intersection(&other.0).cloned().collect())
        }

        fn minus(&self, other: &Self) -> Self {
            Self(self.0.difference(&other.0).cloned().collect())
        }

        fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        fn is_subset(&self, other: &Self) -> bool {
            self.0.difference(&other.0).next().is_none()
        }
    }

    #[test]
    pub fn test_symbolic_counter() {
        let set = TestSet::new(&[1, 3, 4, 5, 6, 8, 9]);
        let mut counter = SymbolicCounter::new(set);

        counter.increment(&TestSet::new(&[1, 3, 4]));
        counter.increment(&TestSet::new(&[4, 6, 8]));

        assert_eq!(counter.max_count(), (2, &TestSet::new(&[4])));
        assert_eq!(counter.min_count(), (0, &TestSet::new(&[5, 9])));

        counter.increment(&TestSet::new(&[3, 4, 6]));
        let counts = counter.export_counts();

        assert_eq!(counts.len(), 4);
        assert_eq!(counts[0], (0, TestSet::new(&[5, 9])));
        assert_eq!(counts[1], (1, TestSet::new(&[1, 8])));
        assert_eq!(counts[2], (2, TestSet::new(&[3, 6])));
        assert_eq!(counts[3], (3, TestSet::new(&[4])));
    }
}
