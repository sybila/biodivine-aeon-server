use crate::bdt::{entropy, information_gain, AppliedAttribute, Attribute, AttributeId, AttributeIds, BDTNode, BDTNodeId, BDTNodeIds, BDT, BifurcationFunction};
use crate::scc::Class;
use crate::util::functional::Functional;
use crate::util::index_type::IndexType;
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use std::collections::{HashMap, HashSet};

impl BDT {
    /// Create a new single-node tree for given classification and attributes.
    pub fn new(classes: BifurcationFunction, attributes: Vec<Attribute>) -> BDT {
        BDT {
            attributes,
            storage: HashMap::new(),
            next_id: 0,
        }
        .apply(|t| t.insert_node_with_classes(classes))
    }

    /// Node ID of the tree root.
    pub fn root_id(&self) -> BDTNodeId {
        BDTNodeId(0)
    }

    /// Iterator over all valid node ids in this tree.
    pub fn nodes(&self) -> BDTNodeIds {
        self.storage.keys().map(|x| BDTNodeId(*x))
    }

    /// Iterator over all attribute ids in this tree.
    pub fn attributes(&self) -> AttributeIds {
        (0..self.attributes.len()).map(|x| AttributeId(x))
    }

    /// Get leaf parameter set if the given node is a leaf.
    pub fn params_for_leaf(&self, node: BDTNodeId) -> Option<&GraphColors> {
        if let BDTNode::Leaf { params, .. } = &self[node] {
            Some(params)
        } else {
            None
        }
    }

    /// **(internal)** Get next available node id in this tree.
    fn next_id(&mut self) -> BDTNodeId {
        BDTNodeId(self.next_id).also(|_| self.next_id += 1)
    }

    /// **(internal)** Replace an EXISTING node in this tree.
    pub(super) fn replace_node(&mut self, id: BDTNodeId, node: BDTNode) {
        if self.storage.insert(id.0, node).is_none() {
            panic!("Replaced a non-existing node.");
        }
    }

    /// **(internal)** Save the given node in this tree and assign it a node id.
    pub(super) fn insert_node(&mut self, node: BDTNode) -> BDTNodeId {
        self.next_id().also(|id| {
            if self.storage.insert(id.0, node).is_some() {
                panic!("Inserted a duplicate node.");
            }
        })
    }

    /// **(internal)** Create a leaf/unprocessed node for the given class list.
    pub(super) fn insert_node_with_classes(
        &mut self,
        classes: BifurcationFunction,
    ) -> BDTNodeId {
        assert!(!classes.is_empty(), "Inserting empty node.");
        return if classes.len() == 1 {
            let (class, params) = classes.into_iter().next().unwrap();
            self.insert_node(BDTNode::Leaf { class, params })
        } else {
            self.insert_node(BDTNode::Unprocessed { classes })
        };
    }

    /// Compute the list of applied attributes (sorted by information gain) for a given node.
    pub fn applied_attributes(&self, node: BDTNodeId) -> Vec<AppliedAttribute> {
        let classes: HashMap<Class, GraphColors> = match &self[node] {
            BDTNode::Leaf { .. } => HashMap::new(),
            BDTNode::Decision { classes, .. } => classes.clone(),
            BDTNode::Unprocessed { classes, .. } => classes.clone(),
        };
        if classes.is_empty() {
            return Vec::new();
        }
        let original_entropy = entropy(&classes);
        self.attributes()
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
            })
    }

    /// Replace an unprocessed node with a decision node using the given attribute.
    pub fn make_decision(
        &mut self,
        node: BDTNodeId,
        attribute: AttributeId,
    ) -> Result<(BDTNodeId, BDTNodeId), String> {
        if !self.storage.contains_key(&node.to_index()) {
            return Err(format!("Node not found."));
        }
        if attribute.to_index() >= self.attributes.len() {
            return Err(format!("Attribute not found"));
        }
        if let BDTNode::Unprocessed { classes } = &self[node] {
            let attr = &self[attribute];
            let (left, right) = attr.split_function(classes);
            if left.is_empty() || right.is_empty() {
                return Err(format!("No decision based on given attribute."));
            }
            let classes = classes.clone();
            let left_node = self.insert_node_with_classes(left);
            let right_node = self.insert_node_with_classes(right);
            self.replace_node(
                node,
                BDTNode::Decision {
                    classes,
                    attribute,
                    left: left_node,
                    right: right_node,
                },
            );
            Ok((left_node, right_node))
        } else {
            Err(format!("Cannot make decision on a resolved node."))
        }
    }

    /// Replace given decision node with an unprocessed node and delete all child nodes.
    ///
    /// Returns list of deleted nodes.
    pub fn revert_decision(&mut self, node: BDTNodeId) -> Vec<BDTNodeId> {
        let mut deleted = vec![];
        if let BDTNode::Decision { classes, .. } = self[node].clone() {
            let mut dfs = vec![node];
            while let Some(expand) = dfs.pop() {
                if let BDTNode::Decision { left, right, .. } = &self[expand] {
                    deleted.push(*left);
                    deleted.push(*right);
                    dfs.push(*left);
                    dfs.push(*right);
                }
            }
            deleted.iter().for_each(|n| {
                self.storage.remove(&n.to_index());
            });
            self.replace_node(node, BDTNode::Unprocessed { classes });
        }
        deleted
    }

    /// Automatically expands all unprocessed nodes with the first (best) decision attribute
    /// up to the given `depth`.
    ///
    /// Returns the list of changed node ids.
    pub fn auto_expand(&mut self, node: BDTNodeId, depth: usize) -> Vec<BDTNodeId> {
        let mut changed = HashSet::new();
        self.auto_expand_recursive(&mut changed, node, depth);
        changed.into_iter().collect()
    }

    fn auto_expand_recursive(&mut self, changed: &mut HashSet<BDTNodeId>, node: BDTNodeId, depth: usize) {
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
        if let BDTNode::Decision { left, right, .. } = &self[node] {
            let (left, right) = (*left, *right);
            self.auto_expand_recursive(changed, left, depth - 1);
            self.auto_expand_recursive(changed, right, depth - 1);
        }
        // Leaves are ignored...
    }

}
