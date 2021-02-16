use crate::bdt::BdtNode;
use crate::scc::Class;
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use std::collections::HashMap;

impl BdtNode {
    /// Computes the cardinality of the parameter set covered by this tree node.
    pub fn approx_cardinality(&self) -> f64 {
        match self {
            BdtNode::Leaf { params, .. } => params.approx_cardinality(),
            BdtNode::Decision { classes, .. } => class_list_cardinality(classes),
            BdtNode::Unprocessed { classes, .. } => class_list_cardinality(classes),
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, BdtNode::Leaf { .. })
    }

    pub fn is_decision(&self) -> bool {
        matches!(self, BdtNode::Decision { .. })
    }

    pub fn is_unprocessed(&self) -> bool {
        matches!(self, BdtNode::Unprocessed { .. })
    }
}

/// **(internal)** Utility method for computing cardinality of a collection of classes.
pub(super) fn class_list_cardinality(classes: &HashMap<Class, GraphColors>) -> f64 {
    classes
        .iter()
        .fold(0.0, |a, (_, b)| a + b.approx_cardinality())
}
