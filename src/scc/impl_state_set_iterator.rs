use super::StateSetIterator;
use crate::scc::StateSetIntoIterator;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_param_bn::biodivine_std::structs::IdState;

impl<'a> Iterator for StateSetIterator<'a> {
    type Item = (IdState, &'a BddParams);

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.set.0.len() {
            if let Some(item) = &self.set.0[self.next] {
                self.next += 1;
                return Some((IdState::from(self.next - 1), item));
            }
            self.next += 1;
        }
        return None;
    }
}

impl Iterator for StateSetIntoIterator {
    type Item = (IdState, BddParams);

    fn next(&mut self) -> Option<Self::Item> {
        self.next += 1;
        let mut item = self.set.next();
        while item == Some(None) {
            self.next += 1;
            item = self.set.next();
        }
        return if let Some(Some(params)) = item {
            Some((IdState::from(self.next - 1), params))
        } else {
            None
        };
    }
}
