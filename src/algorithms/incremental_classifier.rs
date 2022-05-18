use biodivine_lib_param_bn::biodivine_std::traits::Set;

/// Features are objects that are tracked using an `IncrementalClassifier`. Features
/// are composable using the `extend` method. Furthermore, we assume that there is a `Default`
/// value that represents a "zero" with respect to this `extend` operation.
pub trait Feature: Clone + Default + Eq {
    /// Extends this feature using the provided feature.
    fn extend(&self, other: &Self) -> Self;
}

#[derive(Clone)]
pub struct IncrementalClassifier<F: Feature, S: Set> {
    // Sorted vector of features currently tracked by this classifier.
    items: Vec<(F, S)>,
}

impl<F: Feature, S: Set> IncrementalClassifier<F, S> {
    /// Create an empty classifier using `F::default` as the initial feature assigned
    /// to the whole `unit_set`.
    pub fn empty(unit_set: S) -> IncrementalClassifier<F, S> {
        IncrementalClassifier {
            items: vec![(F::default(), unit_set)],
        }
    }

    /// Returns the number of unique features currently tracked by this classifier.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Get the underlying list of features tracked by  this `IncrementalClassifier`.
    ///
    /// Note that the features may be ordered arbitrarily.
    pub fn get_features(&self) -> &[(F, S)] {
        &self.items
    }

    /// Update this `IncrementalClassifier` by extending its values with the given `feature`
    /// for this particular `set` of values.
    pub fn extend(&mut self, feature: &F, set: &S) {
        let mut new_items = Vec::new();

        // Inserts a feature into a list in linear time.
        fn add<F: Feature, S: Set>(items: &mut Vec<(F, S)>, item: (F, S)) {
            if item.1.is_empty() {
                return;
            }

            for (f, s) in items.iter_mut() {
                if f == &item.0 {
                    *s = s.union(&item.1);
                    return;
                }
            }
            items.push(item);
        }

        let mut remaining = set.clone();
        // Since we are copying the items into a new array, there is no danger of counting
        // the same item multiple times.
        for (f, s) in self.items.iter() {
            add(&mut new_items, (f.clone(), s.minus(&remaining)));
            add(&mut new_items, (f.extend(feature), s.intersect(&remaining)));
            remaining = remaining.minus(&s);
        }

        assert!(remaining.is_empty());
        self.items = new_items;
    }
}
