use super::{Class, Behaviour};

impl Class {

    pub fn new_empty() -> Class {
        return Class(Vec::new());
    }

    pub fn extend(&mut self, behaviour: Behaviour) {
        self.0.push(behaviour);
        self.0.sort();
    }

}