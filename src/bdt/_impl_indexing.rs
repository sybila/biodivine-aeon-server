use crate::bdt::{Attribute, AttributeId, BDTNode, BDTNodeId, BDT};
use crate::util::functional::Functional;
use crate::util::index_type::IndexType;
use std::fmt::{Display, Formatter};
use std::ops::Index;

impl IndexType<BDTNode, BDT> for BDTNodeId {
    fn to_index(&self) -> usize {
        self.0
    }

    fn try_from(index: usize, collection: &BDT) -> Option<Self> {
        BDTNodeId(index).take_if(|i| collection.storage.contains_key(&i.0))
    }
}

impl IndexType<Attribute, BDT> for AttributeId {
    fn to_index(&self) -> usize {
        self.0
    }

    fn try_from(index: usize, collection: &BDT) -> Option<Self> {
        AttributeId(index).take_if(|i| i.0 < collection.attributes.len())
    }
}

impl Index<BDTNodeId> for BDT {
    type Output = BDTNode;

    fn index(&self, index: BDTNodeId) -> &Self::Output {
        &self.storage[&index.to_index()]
    }
}

impl Index<AttributeId> for BDT {
    type Output = Attribute;

    fn index(&self, index: AttributeId) -> &Self::Output {
        &self.attributes[index.to_index()]
    }
}

impl Display for BDTNodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl Display for AttributeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}
