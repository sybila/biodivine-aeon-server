use crate::bdt::{
    entropy, information_gain, AppliedAttribute, Attribute, AttributeId, AttributeIds, Bdt,
    BdtNode, BdtNodeId, BdtNodeIds, BifurcationFunction,
};
use crate::scc::Class;
use crate::util::functional::Functional;
use crate::util::index_type::IndexType;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use std::collections::{HashMap, HashSet};
use std::option::Option::Some;

impl Bdt {
    /// Create a new single-node tree for given classification and attributes.
    pub fn new(classes: BifurcationFunction, attributes: Vec<Attribute>) -> Bdt {
        Bdt {
            attributes,
            storage: HashMap::new(),
            next_id: 0,
            precision: None,
        }
        .apply(|t| t.insert_node_with_classes(classes))
    }

    /// Sets the precision of this tree in the hundreds of percent (i.e. 9753 becomes 97.53%).
    ///
    /// If the value truncated to 50-100%.
    ///
    /// Note that while here, precision is exact, the final process involves floating point
    /// math and can be therefore inexact.
    pub fn set_precision(&mut self, precision: u32) {
        if precision < 5001 {
            self.precision = Some(5001);
        } else if precision >= 10000 {
            self.precision = None;
        } else {
            self.precision = Some(precision);
        }
    }

    pub fn get_precision(&self) -> u32 {
        if let Some(precision) = self.precision {
            precision
        } else {
            10000
        }
    }

    /// Node ID of the tree root.
    pub fn root_id(&self) -> BdtNodeId {
        BdtNodeId(0)
    }

    /// Iterator over all valid node ids in this tree.
    pub fn nodes(&self) -> BdtNodeIds {
        self.storage.keys().map(|x| BdtNodeId(*x))
    }

    /// Iterator over all attribute ids in this tree.
    pub fn attributes(&self) -> AttributeIds {
        (0..self.attributes.len()).map(AttributeId)
    }

    /// Get leaf parameter set if the given node is a leaf.
    pub fn params_for_leaf(&self, node: BdtNodeId) -> Option<&GraphColors> {
        if let BdtNode::Leaf { params, .. } = &self[node] {
            Some(params)
        } else {
            None
        }
    }

    /// Compute all parameters that are stored in the given tree node.
    pub fn all_node_params(&self, node: BdtNodeId) -> GraphColors {
        match &self[node] {
            BdtNode::Leaf { params, .. } => params.clone(),
            BdtNode::Unprocessed { classes, .. } => Self::class_union(classes),
            BdtNode::Decision { classes, .. } => Self::class_union(classes),
        }
    }

    fn class_union(classes: &BifurcationFunction) -> GraphColors {
        let mut iterator = classes.values();
        let mut result = iterator.next().unwrap().clone();
        for value in iterator {
            result = result.union(value)
        }
        result
    }

    /// **(internal)** Get next available node id in this tree.
    fn next_id(&mut self) -> BdtNodeId {
        BdtNodeId(self.next_id).also(|_| self.next_id += 1)
    }

    /// **(internal)** Replace an EXISTING node in this tree.
    pub(super) fn replace_node(&mut self, id: BdtNodeId, node: BdtNode) {
        if self.storage.insert(id.0, node).is_none() {
            panic!("Replaced a non-existing node.");
        }
    }

    /// **(internal)** Save the given node in this tree and assign it a node id.
    pub(super) fn insert_node(&mut self, node: BdtNode) -> BdtNodeId {
        self.next_id().also(|id| {
            if self.storage.insert(id.0, node).is_some() {
                panic!("Inserted a duplicate node.");
            }
        })
    }

    /// **(internal)** Create a leaf/unprocessed node for the given class list.
    pub(super) fn insert_node_with_classes(&mut self, classes: BifurcationFunction) -> BdtNodeId {
        assert!(!classes.is_empty(), "Inserting empty node.");
        if classes.len() == 1 {
            let (class, params) = classes.into_iter().next().unwrap();
            self.insert_node(BdtNode::Leaf { class, params })
        } else {
            self.insert_node(BdtNode::Unprocessed { classes })
        }
    }

    /// Compute the list of applied attributes (sorted by information gain) for a given node.
    pub fn applied_attributes(&self, node: BdtNodeId) -> Vec<AppliedAttribute> {
        let classes: HashMap<Class, GraphColors> = match &self[node] {
            BdtNode::Leaf { .. } => HashMap::new(),
            BdtNode::Decision { classes, .. } => classes.clone(),
            BdtNode::Unprocessed { classes, .. } => classes.clone(),
        };
        if classes.is_empty() {
            return Vec::new();
        }
        let original_entropy = entropy(&classes);
        let attributes = self
            .attributes()
            .filter_map(|id| {
                let attribute = &self[id];
                let (left, right) = attribute.split_function(&classes);
                let gain = information_gain(original_entropy, entropy(&left), entropy(&right));
                AppliedAttribute {
                    attribute: id,
                    information_gain: gain,
                    left,
                    right,
                }
                .take_if(|it| it.information_gain > f64::NEG_INFINITY)
            })
            .collect::<Vec<_>>()
            .apply(|it| {
                it.sort_by(|l, r| l.information_gain.partial_cmp(&r.information_gain).unwrap());
                it.reverse();
            });

        Vec::new().apply(|result| {
            for a in &attributes {
                let attr_a = &self[a.attribute];
                let mut skip = false;
                for b in &attributes {
                    let attr_b = &self[b.attribute];
                    if attr_b.is_specification_of(attr_a) {
                        skip |= a.left == b.left && a.right == b.right;
                    }
                }
                if !skip {
                    result.push(a.clone());
                }
            }
        })
    }

    /// Replace an unprocessed node with a decision node using the given attribute.
    pub fn make_decision(
        &mut self,
        node: BdtNodeId,
        attribute: AttributeId,
    ) -> Result<(BdtNodeId, BdtNodeId), String> {
        if !self.storage.contains_key(&node.to_index()) {
            return Err("Node not found.".to_string());
        }
        if attribute.to_index() >= self.attributes.len() {
            return Err("Attribute not found".to_string());
        }
        if let BdtNode::Unprocessed { classes } = &self[node] {
            let attr = &self[attribute];
            let (left, right) = attr.split_function(classes);
            if left.is_empty() || right.is_empty() {
                return Err("No decision based on given attribute.".to_string());
            }
            let classes = classes.clone();
            let left_node = self.insert_node_with_classes(left);
            let right_node = self.insert_node_with_classes(right);
            self.replace_node(
                node,
                BdtNode::Decision {
                    classes,
                    attribute,
                    left: left_node,
                    right: right_node,
                },
            );
            Ok((left_node, right_node))
        } else {
            Err("Cannot make decision on a resolved node.".to_string())
        }
    }

    /// Replace given decision node with an unprocessed node and delete all child nodes.
    ///
    /// Returns list of deleted nodes.
    pub fn revert_decision(&mut self, node: BdtNodeId) -> Vec<BdtNodeId> {
        let mut deleted = vec![];
        if let BdtNode::Decision { classes, .. } = self[node].clone() {
            let mut dfs = vec![node];
            while let Some(expand) = dfs.pop() {
                if let BdtNode::Decision { left, right, .. } = &self[expand] {
                    deleted.push(*left);
                    deleted.push(*right);
                    dfs.push(*left);
                    dfs.push(*right);
                }
            }
            deleted.iter().for_each(|n| {
                self.storage.remove(&n.to_index());
            });
            self.replace_node(node, BdtNode::Unprocessed { classes });
        }
        deleted
    }

    /// Automatically expands all unprocessed nodes with the first (best) decision attribute
    /// up to the given `depth`.
    ///
    /// Returns the list of changed node ids.
    pub fn auto_expand(&mut self, node: BdtNodeId, depth: u32) -> HashSet<BdtNodeId> {
        let mut changed = HashSet::new();
        self.auto_expand_recursive(&mut changed, node, depth);
        changed
    }

    fn auto_expand_recursive(
        &mut self,
        changed: &mut HashSet<BdtNodeId>,
        node: BdtNodeId,
        depth: u32,
    ) {
        if depth == 0 {
            return;
        }
        // If this is unprocessed node, make a default decision.
        if self[node].is_unprocessed() {
            let attr = self.applied_attributes(node).into_iter().next();
            if let Some(attr) = attr {
                let (left, right) = self.make_decision(node, attr.attribute).unwrap();
                changed.insert(node);
                changed.insert(left);
                changed.insert(right);
                self.auto_expand_recursive(changed, left, depth - 1);
                self.auto_expand_recursive(changed, right, depth - 1);
            } else {
                return; // No attributes, no fun...
            }
        }
        // For expanded nodes, just follow.
        if let BdtNode::Decision { left, right, .. } = &self[node] {
            let (left, right) = (*left, *right);
            self.auto_expand_recursive(changed, left, depth - 1);
            self.auto_expand_recursive(changed, right, depth - 1);
        }
        // Leaves are ignored...
    }
}
