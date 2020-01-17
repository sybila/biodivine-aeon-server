use super::StateSetIterator;
use biodivine_lib_param_bn::bdd_params::BddParams;
use biodivine_lib_std::IdState;

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
