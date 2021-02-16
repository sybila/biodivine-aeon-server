use super::{Behaviour, Class};
use std::cmp::Ordering;
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

    pub fn get_vector(&self) -> Vec<Behaviour> {
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

/// Classes actually have a special ordering - primarily, they are ordered by the
/// number of behaviours, secondarily they are ordered by the actual behaviours.
impl PartialOrd for Class {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return if self.0.len() != other.0.len() {
            self.0.len().partial_cmp(&other.0.len())
        } else {
            if self.0.len() == 0 {
                Some(Ordering::Equal)
            } else {
                self.0.partial_cmp(&other.0)
            }
        };
    }
}
