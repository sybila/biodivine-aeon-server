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

    pub fn clone_extended(&self, behaviour: Behaviour) -> Class {
        let mut vec = self.0.clone();
        vec.push(behaviour);
        vec.sort();
        return Class(vec);
    }

    pub fn to_vec(&self) -> Vec<Behaviour> {
        self.0.clone()
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        return write!(
            f,
            "{:?}",
            self.0
                .iter()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<_>>()
        );
    }
}
