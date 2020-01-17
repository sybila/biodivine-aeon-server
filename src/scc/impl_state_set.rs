use super::{StateSet, StateSetIterator};
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::param_graph::Params;
use biodivine_lib_std::IdState;

impl StateSet {
    pub fn new(capacity: usize) -> StateSet {
        return StateSet(vec![None; capacity]);
    }

    pub fn new_with_initial(capacity: usize, default: &BddParams) -> StateSet {
        return StateSet(vec![Some(default.clone()); capacity]);
    }

    pub fn new_with_fun<F>(capacity: usize, mut init: F) -> StateSet
    where
        F: FnMut(IdState) -> Option<BddParams>,
    {
        let mut data = Vec::new();
        for i in 0..capacity {
            let p = init(IdState::from(i));
            if let Some(p) = p {
                if !p.is_empty() {
                    data.push(Some(p));
                } else {
                    data.push(None);
                }
            } else {
                data.push(None);
            }
        }
        return StateSet(data);
    }

    pub fn capacity(&self) -> usize {
        return self.0.len();
    }

    pub fn get(&self, state: IdState) -> Option<&BddParams> {
        let index: usize = state.into();
        return if let Some(params) = &self.0[index] {
            Some(params)
        } else {
            None
        };
    }

    pub fn get_mut(&mut self, state: IdState) -> &mut Option<BddParams> {
        let index: usize = state.into();
        return &mut self.0[index];
    }

    pub fn put(&mut self, state: IdState, params: BddParams) {
        let index: usize = state.into();
        self.0[index] = Some(params);
    }

    pub fn clear_key(&mut self, state: IdState) {
        let index: usize = state.into();
        self.0[index] = None;
    }

    pub fn union_key(&mut self, state: IdState, params: &BddParams) -> bool {
        let value = self.get_mut(state);
        return if let Some(current) = value {
            if params.is_subset(current) {
                false
            } else {
                *value = Some(current.union(params));
                true
            }
        } else {
            *value = Some(params.clone());
            true
        };
    }

    pub fn intersect_key(&mut self, state: IdState, params: &BddParams) {
        if let Some(current) = self.get(state) {
            let new = current.intersect(params);
            self.put(state, new);
        }
    }

    pub fn subtract_key(&mut self, state: IdState, params: &BddParams) {
        if let Some(current) = self.get(state) {
            let result = current.minus(params);
            if result.is_empty() {
                self.clear_key(state);
            } else {
                self.put(state, result);
            }
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        return self.element_binary_op(other, |a, b| match (a, b) {
            (Some(a), Some(b)) => Some(a.intersect(b)),
            _ => None,
        });
    }

    pub fn union(&self, other: &Self) -> Self {
        return self.element_binary_op(other, |a, b| match (a, b) {
            (Some(a), Some(b)) => Some(a.union(b)),
            (Some(a), _) => Some(a.clone()),
            (_, Some(b)) => Some(b.clone()),
            _ => None,
        });
    }

    pub fn minus(&self, other: &Self) -> Self {
        return self.element_binary_op(other, |a, b| match (a, b) {
            (Some(a), Some(b)) => Some(a.minus(b)),
            (Some(a), _) => Some(a.clone()),
            _ => None,
        });
    }

    pub fn element_binary_op<F>(&self, other: &Self, op: F) -> Self
    where
        F: Fn(&Option<BddParams>, &Option<BddParams>) -> Option<BddParams>,
    {
        if self.0.len() != other.0.len() {
            panic!("Incompatible state sets!");
        }
        let mut result = Vec::with_capacity(self.0.len());
        for i in 0..self.0.len() {
            let params = op(&self.0[i], &other.0[i]).filter(|p| !p.is_empty());
            result.push(params);
        }
        return StateSet(result);
    }

    pub fn iter(&self) -> StateSetIterator {
        return StateSetIterator { set: self, next: 0 };
    }

    pub fn fold_union(&self) -> Option<BddParams> {
        return self.iter().fold(None, |a, (_, b)| {
            if let Some(a) = a {
                Some(a.union(b))
            } else {
                Some(b.clone())
            }
        });
    }

    // just for debugging
    pub(crate) fn cardinalities(&self) -> Vec<(usize, f64)> {
        return self
            .iter()
            .map(|(s, p)| (s.into(), p.cardinality()))
            .collect();
    }
}