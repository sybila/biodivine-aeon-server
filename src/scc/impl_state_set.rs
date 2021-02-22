use super::{StateSet, StateSetIterator};
use crate::scc::StateSetIntoIterator;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_param_bn::biodivine_std::structs::IdState;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use rayon::prelude::*;

impl StateSet {
    pub fn new(capacity: usize) -> StateSet {
        return StateSet(vec![None; capacity]);
    }

    pub fn new_with_initial(capacity: usize, default: &BddParams) -> StateSet {
        return StateSet(vec![Some(default.clone()); capacity]);
    }

    pub fn new_with_fun<F>(capacity: usize, init: F) -> StateSet
    where
        F: Fn(IdState) -> Option<BddParams> + Send + Sync,
    {
        let mut data = vec![None; capacity];
        let data_pairs: Vec<(usize, BddParams)> = (0..capacity)
            .collect::<Vec<_>>()
            .par_iter()
            .filter_map(|i: &usize| {
                let p = init(IdState::from(*i));
                if let Some(p) = p {
                    if !p.is_empty() {
                        Some((*i, p))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for (i, p) in data_pairs {
            data[i] = Some(p);
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
        if params.is_empty() {
            self.0[index] = None;
        } else {
            self.0[index] = Some(params);
        }
    }

    pub fn clear_key(&mut self, state: IdState) {
        let index: usize = state.into();
        self.0[index] = None;
    }

    pub fn union_key(&mut self, state: IdState, params: &BddParams) -> bool {
        let value = self.get_mut(state);
        return if let Some(current) = value {
            let new = current.union(params);
            if new.eq(current) {
                // we can abuse the fact that Bdd-s are canonical
                false
            } else {
                *value = Some(new);
                true
            }
        /*if params.is_subset(current) {
            false
        } else {
            *value = Some(current.union(params));
            true
        }*/
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

    pub fn minus_key(&mut self, state: IdState, params: &BddParams) {
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
            (Some(a), Some(b)) => {
                let result = a.intersect(b);
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }
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
            (Some(a), Some(b)) => {
                let result = a.minus(b);
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }
            (Some(a), _) => Some(a.clone()),
            _ => None,
        });
    }

    pub fn element_binary_op<F>(&self, other: &Self, op: F) -> Self
    where
        F: Fn(&Option<BddParams>, &Option<BddParams>) -> Option<BddParams> + Send + Sync,
    {
        if self.0.len() != other.0.len() {
            panic!("Incompatible state sets!");
        }
        let mut result = vec![None; self.0.len()];
        let value_pairs: Vec<(usize, BddParams)> = (0..self.0.len())
            .collect::<Vec<_>>()
            .par_iter()
            .filter_map(|i: &usize| {
                op(&self.0[*i], &other.0[*i])
                    .filter(|p| !p.is_empty())
                    .map(|p| (*i, p))
            })
            .collect::<Vec<_>>();
        for (i, p) in value_pairs {
            result[i] = Some(p);
        }
        return StateSet(result);
    }

    pub fn iter(&self) -> StateSetIterator {
        return StateSetIterator { set: self, next: 0 };
    }

    pub fn into_iter(self) -> StateSetIntoIterator {
        return StateSetIntoIterator {
            set: self.0.into_iter(),
            next: 0,
        };
    }

    pub fn minus_in_place(&mut self, other: &Self) {
        for i in 0..self.0.len() {
            if let Some(other) = &other.0[i] {
                let current = &mut self.0[i];
                if let Some(current) = current.as_mut() {
                    let new = current.minus(other);
                    *current = new;
                }
            } // else its none, dont minus anything
        }
    }

    /// Intersect all values in this state set with the given params (in parallel).
    pub fn par_restrict_to_params(&mut self, params: &BddParams) {
        self.0
            .par_iter_mut()
            .for_each(|value: &mut Option<BddParams>| {
                if value.is_some() {
                    let new_params = params.intersect(value.as_ref().unwrap());
                    if new_params.is_empty() {
                        *value = None;
                    } else {
                        *value = Some(new_params);
                    }
                }
            });
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

    pub(crate) fn _cardinalities(&self) -> Vec<(usize, f64)> {
        return self
            .iter()
            .map(|(s, p)| (s.into(), p.cardinality()))
            .collect();
    }
}
