use crate::bdt::{Attribute, AttributeId, Bdt, BdtNode, BdtNodeId};
use crate::util::functional::Functional;
use std::fmt::{Display, Formatter};
use std::ops::Index;

impl BdtNodeId {
    pub fn to_index(&self) -> usize {
        self.0
    }

    pub fn try_from(index: usize, collection: &Bdt) -> Option<Self> {
        BdtNodeId(index).take_if(|i| collection.storage.contains_key(&i.0))
    }

    pub fn try_from_str(string: &str, collection: &Bdt) -> Option<Self> {
        let index = string.parse::<usize>().ok()?;
        Self::try_from(index, collection)
    }
}

impl AttributeId {
    pub fn to_index(&self) -> usize {
        self.0
    }

    pub fn try_from(index: usize, collection: &Bdt) -> Option<Self> {
        AttributeId(index).take_if(|i| i.0 < collection.attributes.len())
    }

    pub fn try_from_str(string: &str, collection: &Bdt) -> Option<Self> {
        let index = string.parse::<usize>().ok()?;
        AttributeId::try_from(index, collection)
    }
}

impl Index<BdtNodeId> for Bdt {
    type Output = BdtNode;

    fn index(&self, index: BdtNodeId) -> &Self::Output {
        &self.storage[&index.to_index()]
    }
}

impl Index<AttributeId> for Bdt {
    type Output = Attribute;

    fn index(&self, index: AttributeId) -> &Self::Output {
        &self.attributes[index.to_index()]
    }
}

impl Display for BdtNodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl Display for AttributeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}
