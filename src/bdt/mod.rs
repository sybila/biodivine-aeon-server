use crate::scc::Class;
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::iter::Map;
use std::ops::Range;

/// **(internal)** All necessary building blocks for computing a list of attributes from a
/// Boolean network.
mod _attributes_for_network;
/// **(internal)** Some utility functions for working with attributes.
mod _impl_attribute;
/// **(internal)** Implementation of utility methods for the binary decision tree.
mod _impl_bdt;
/// **(internal)** Implementation of .dot export utilities for a decision tree.
mod _impl_bdt_dot_export;
/// **(internal)** Implementation of json serialization of BDT structures.
mod _impl_bdt_json;
/// **(internal)** Implementation of general convenience methods for BDT nodes.
mod _impl_bdt_node;
/// **(internal)** Implementation of indexing operations provided by BDTNodeId and AttributeId.
mod _impl_indexing;

type BifurcationFunction = HashMap<Class, GraphColors>;

/// Encodes one node of a bifurcation decision tree. A node can be either a leaf (fully classified
/// set of parametrisations), a decision node with a fixed attribute, or an unprocessed node
/// with a remaining bifurcation function.
#[derive(Clone)]
pub enum BDTNode {
    Leaf {
        class: Class,
        params: GraphColors,
    },
    Decision {
        attribute: AttributeId,
        left: BDTNodeId,
        right: BDTNodeId,
        classes: BifurcationFunction,
    },
    Unprocessed {
        classes: BifurcationFunction,
    },
}

/// An identifier of a BDT node. These are used to quickly refer to parts of a BDT, for example
/// from GUI.
///
/// I might want to delete a node - to avoid specifying a full path from root to the deleted node,
/// I can use the ID which will automatically "jump" to the correct position in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BDTNodeId(usize);

/// An attribute id is used to identify a specific attribute used in a decision tree.
///
/// These are bound to a specific BDT, but note that not all attributes have to be applicable
/// to all BDT nodes (or, more specifically, they are applicable but have no effect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttributeId(usize);

/// A Bifurcation decision tree. It stores the BDT nodes, mapping IDs to actual structures.
///
/// It is the owner of the tree memory, so every addition/deletion in the tree must happen here.
pub struct BDT {
    storage: HashMap<usize, BDTNode>,
    attributes: Vec<Attribute>,
    next_id: usize,
}

type BDTNodeIds<'a> = Map<Keys<'a, usize, BDTNode>, fn(&usize) -> BDTNodeId>;
type AttributeIds<'a> = Map<Range<usize>, fn(usize) -> AttributeId>;

/// Attribute is an abstract property of the boolean network that can be applied to partition
/// the parameter space into two sub-spaces.
#[derive(Clone)]
pub struct Attribute {
    name: String,
    positive: GraphColors,
    negative: GraphColors,
}

/// A small helper struct that represents the data produced when an attribute is applied to
/// a given bifurcation function.
///
/// (Right now, there are almost big picture plans for the API of this thing, so it is left
/// public with almost no support, but maybe we'll come up with something later)
pub struct AppliedAttribute {
    pub attribute: AttributeId,
    pub left: BifurcationFunction,
    pub right: BifurcationFunction,
    pub information_gain: f64,
}

/// Compute entropy of the behaviour class data set
pub fn entropy(classes: &BifurcationFunction) -> f64 {
    if classes.is_empty() {
        return f64::INFINITY;
    }
    let mut result = 0.0;
    let cardinality: Vec<f64> = classes.values().map(|it| it.approx_cardinality()).collect();
    let total = cardinality.iter().fold(0.0, |a, b| a + *b);
    for c in cardinality.iter() {
        let proportion = *c / total;
        result += -proportion * proportion.log2();
    }
    return result;
}

/// Complete information gain from original and divided dataset cardinality.
pub fn information_gain(original: f64, left: f64, right: f64) -> f64 {
    original - (0.5 * left + 0.5 * right)
}
