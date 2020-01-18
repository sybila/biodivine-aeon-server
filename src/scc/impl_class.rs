use super::{Behaviour, Class};
use std::fmt::{Display, Error, Formatter};

impl Class {
    pub fn new_empty() -> Class {
        return Class(Vec::new());
    }

    pub fn extend(&mut self, behaviour: Behaviour) {
        self.0.push(behaviour);
        self.0.sort();
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        return write!(f, "\"{:?}\"", self.0);
    }
}
