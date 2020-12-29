use crate::bdt::_impl_bdt_node::class_list_cardinality;
use crate::bdt::{AttributeId, BDTNode, BDTNodeId, BDT};
use crate::scc::Class;
use crate::util::functional::Functional;
use crate::util::index_type::IndexType;
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use json::JsonValue;
use std::collections::HashMap;

impl BDTNode {
    /// Convert this BDT node to json value with all available information stored in the node.
    pub fn to_json(&self) -> JsonValue {
        match self {
            BDTNode::Leaf { class, params } => object! {
                "type" => format!("leaf"),
                "cardinality" => params.approx_cardinality(),
                "class" => format!("{}", class),
            },
            BDTNode::Unprocessed { classes } => object! {
                "type" => format!("unprocessed"),
                "cardinality" => class_list_cardinality(classes),
                "classes" => class_list_to_json(classes),
            },
            BDTNode::Decision {
                attribute,
                left,
                right,
                classes,
            } => object! {
                "type" => format!("decision"),
                "cardinality" => class_list_cardinality(classes),
                "classes" => class_list_to_json(classes),
                "attribute_id" => attribute.0,
                "left" => left.0,
                "right" => right.0,
            },
        }
    }
}

impl BDT {
    /// Convert the whole tree into one json array.
    pub fn to_json(&self) -> JsonValue {
        JsonValue::from(
            self.nodes()
                .map(|id| self.node_to_json(id))
                .collect::<Vec<_>>(),
        )
    }

    /// Convert a BDT node to json, including extra info compared to `BDTNode::to_json`.
    ///
    /// The extra info covers the node id as well as attribute name for decision nodes.
    pub fn node_to_json(&self, id: BDTNodeId) -> JsonValue {
        self[id].to_json().apply(|result| {
            result.insert("id", id.0).unwrap();
            if result.has_key("attribute_id") {
                let attr_id: AttributeId = result["attribute_id"]
                    .as_usize()
                    .and_then(|i| AttributeId::try_from(i, self))
                    .unwrap();
                result
                    .insert("attribute_name", self[attr_id].name.clone())
                    .unwrap();
            }
        })
    }

    /// Compute attribute gains for the given tree node.
    pub fn attribute_gains_json(&self, id: BDTNodeId) -> JsonValue {
        self.applied_attributes(id)
            .into_iter()
            .map(|it| {
                object! {
                    "id" => it.attribute.to_index(),
                    "name" => self[it.attribute].name.clone(),
                    "left" => class_list_to_json(&it.left),
                    "right" => class_list_to_json(&it.right),
                    "gain" => it.information_gain
                }
            })
            .collect::<Vec<_>>()
            .and_then(|it| JsonValue::from(it))
    }
}

pub(super) fn class_list_to_json(classes: &HashMap<Class, GraphColors>) -> JsonValue {
    JsonValue::from(classes.iter().map(class_to_json).collect::<Vec<_>>())
}

pub(super) fn class_to_json((class, params): (&Class, &GraphColors)) -> JsonValue {
    object! {
        "cardinality" => params.approx_cardinality(),
        "class" => format!("{}", class),
    }
}
