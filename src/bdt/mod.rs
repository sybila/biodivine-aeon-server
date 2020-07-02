use crate::scc::Class;
use biodivine_lib_bdd::bdd;
use biodivine_lib_param_bn::bdd_params::{BddParameterEncoder, BddParams, FunctionTableEntry};
use biodivine_lib_param_bn::{BooleanNetwork, VariableId, Variable};
use biodivine_lib_std::param_graph::Params;
use std::collections::HashMap;
use std::cmp::max;

/// Encodes one node of a bifurcation decision tree. A node can be either a leaf (fully classified
/// set of parametrisations), a decision node with a fixed attribute, or an unprocessed node
/// with a remaining bifurcation function.
#[derive(Debug)]
pub enum BDTNode {
    Leaf { class: Class, params: BddParams },
    Decision { attribute: AttributeId, left: BDTNodeId, right: BDTNodeId },
    Unprocessed { classes: HashMap<Class, BddParams> }
}

/// An identifier of a BDT node. These are used to quickly refer to parts of a BDT, for example
/// from GUI.
///
/// I might want to delete a node - to avoid specifying a full path from root to the deleted node,
/// I can use the ID which will automatically "jump" to the correct position in the tree.
#[derive(Debug, Clone, Copy)]
pub struct BDTNodeId(usize);

#[derive(Debug, Clone, Copy)]
pub struct AttributeId(usize);

/// A Bifurcation decision tree. It stores the BDT nodes, mapping IDs to actual structures.
/// It is the owner of the tree memory, so every addition/deletion in the tree must happen here.
pub struct BDT {
    storage: HashMap<usize, BDTNode>,
    attributes: Vec<Attribute>,
    next_id: usize
}

impl BDT {

    pub fn new(classes: HashMap<Class, BddParams>, attributes: Vec<Attribute>) -> BDT {
        let mut tree = BDT { storage: HashMap::new(), attributes, next_id: 0 };
        tree.insert_new_node(classes);
        return tree;
    }

    pub fn root_id() -> BDTNodeId {
        return BDTNodeId(0);
    }

    pub fn is_unprocessed(&self, id: BDTNodeId) -> bool {
        return if let BDTNode::Unprocessed { .. } = self.storage.get(&id.0).unwrap() {
            true
        } else {
            false
        }
    }

    pub fn learn_tree(&mut self, max_depth: usize) {
        let attr: Vec<usize> =  (0..(self.attributes.len())).collect();
        return self.learn_tree_recursive(0, &attr, 0, max_depth);
    }

    pub fn dump_dot(&self) {
        println!("digraph G {{");
        println!("init__ [label=\"\", style=invis, height=0, width=0];");
        println!("init__ -> 0;");
        self.dump_dot_recursive(0);
        println!("}}");
    }

    fn dump_dot_recursive(&self, node: usize) {
        match &self.storage[&node] {
            BDTNode::Leaf { class, params} => {
                println!("{}[label=\"{}({})\"];", node, format!("{}", class).replace("\"", ""), params.cardinality());
            }
            BDTNode::Unprocessed { classes } => {
                let classes: Vec<String> = classes.iter().map(|(c, p)| {
                    format!("({},{})", c, p.cardinality()).replace("\"", "")
                }).collect();
                let classes = format!("{:?}", classes).replace("\"", "");
                println!("{}[label=\"Unprocessed({})\"]", node, classes);
            }
            BDTNode::Decision { attribute, left, right } => {
                let (left, right) = (left.0, right.0);
                let attribute = &self.attributes[attribute.0];
                println!("{}[label=\"{}\"]", node, attribute.name);
                println!("{} -> {} [style=dotted];", node, left);
                println!("{} -> {} [style=filled];", node, right);
                self.dump_dot_recursive(left);
                self.dump_dot_recursive(right);
            }
        }
    }

    fn learn_tree_recursive(&mut self, node: usize, attr: &Vec<usize>, depth: usize, max_depth: usize) {
        if depth >= max_depth { return; }
        match &self.storage[&node] {
            BDTNode::Leaf { .. } => return,     // already processed, skip
            BDTNode::Decision { attribute, left, right } => {
                // Processed, but maybe has unprocessed children!
                let (left, right) = (left.0, right.0);  // hint for borrow checker to release reference to self
                self.learn_tree_recursive(left, attr, depth + 1, max_depth);
                self.learn_tree_recursive(right, attr, depth + 1, max_depth);
            }
            BDTNode::Unprocessed { classes } => {
                // Find best attribute and continue.
                let mut continue_with = Vec::new();
                let mut max: Option<(usize, f64)> = None;
                for a in attr.iter() {
                    let attribute = &self.attributes[*a];
                    let gain = attribute.information_gain(classes);
                    //println!("Gain {} from {}.", gain, attribute.name);
                    if gain > f64::NEG_INFINITY {
                        continue_with.push(*a);
                        if let Some((current, current_gain)) = max {
                            if gain > current_gain {
                                max = Some((*a, gain));
                            }
                        } else {
                            max = Some((*a, gain));
                        }
                    }
                }
                if let Some((max, max_gain)) = max {
                    println!("Select attr: {} with gain {}", self.attributes[max].name, max_gain);
                    let (l, r) = self.make_decision(BDTNodeId(node), AttributeId(max));
                    self.learn_tree_recursive(l.0, &continue_with, depth + 1, max_depth);
                    self.learn_tree_recursive(r.0, &continue_with, depth + 1, max_depth);
                } else {
                    panic!("No suitable attribute found!")
                }
            }
        }
    }

    fn insert_leaf(&mut self, class: Class, params: BddParams) -> BDTNodeId {
        let leaf = BDTNode::Leaf { class, params };
        let leaf_id = self.next_id();
        self.insert_or_replace(leaf_id, leaf, false);
        return BDTNodeId(leaf_id);
    }

    fn insert_unprocessed(&mut self, classes: HashMap<Class, BddParams>) -> BDTNodeId {
        let node = BDTNode::Unprocessed { classes };
        let id = self.next_id();
        self.insert_or_replace(id, node, false);
        return BDTNodeId(id);
    }

    fn insert_new_node(&mut self, classes: HashMap<Class, BddParams>) -> BDTNodeId {
        assert!(!classes.is_empty(), "Inserting empty node.");
        return if classes.len() == 1 {
            let (class, params) = classes.into_iter().next().unwrap();
            self.insert_leaf(class, params)
        } else {
            self.insert_unprocessed(classes)
        }
    }

    fn make_decision(&mut self, node: BDTNodeId, attribute_id: AttributeId) -> (BDTNodeId, BDTNodeId) {
        let id = node.0;
        let node = self.storage.get(&id).expect("Invalid node id.");
        let attribute = self.attributes.get(attribute_id.0).expect("Invalid attribute id.");
        if let BDTNode::Unprocessed { classes } = node {
            let (left_data, right_data) = attribute.restrict(classes);
            assert!(!(left_data.is_empty() || right_data.is_empty()), "No decision based on attribute {}.", attribute.name);
            let left = self.insert_new_node(left_data);
            let right = self.insert_new_node(right_data);
            let decision = BDTNode::Decision {
                attribute: attribute_id, left, right
            };
            self.insert_or_replace(id, decision, true);
            return (left, right);
        } else {
            panic!("Expected unprocessed node.");
        }
    }

    fn insert_or_replace(&mut self, id: usize, node: BDTNode, replace: bool) {
        let old = self.storage.insert(id, node);
        assert_eq!(replace, old.is_some(), "Modifying {:?}, but {:?} already in the tree.", id, old);
    }

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        return id;
    }

}

/// Restrict given classes using the specified attribute parameters
fn apply_attribute(
    classes: &HashMap<Class, BddParams>,
    attribute: &BddParams,
) -> HashMap<Class, BddParams> {
    let mut result = HashMap::new();
    for (c, p) in classes {
        let new_p = attribute.intersect(p);
        if !new_p.is_empty() {
            result.insert(c.clone(), new_p);
        }
    }
    return result;
}

/// Compute entropy of the behaviour class data set
fn entropy(classes: &HashMap<Class, BddParams>) -> f64 {
    if classes.is_empty() {
        return f64::INFINITY;
    }
    let mut result = 0.0;
    let cardinality: Vec<f64> = classes.values().map(|it| it.cardinality()).collect();
    let total = cardinality.iter().fold(0.0, |a, b| a + *b);
    for c in cardinality.iter() {
        let proportion = *c / total;
        result += -proportion * proportion.log2();
    }
    return result;
}


#[derive(Clone)]
pub struct Attribute {
    name: String,
    positive: BddParams,
    negative: BddParams,
}

impl Attribute {

    pub fn restrict(&self, classes: &HashMap<Class, BddParams>) -> (HashMap<Class, BddParams>, HashMap<Class, BddParams>) {
        return (apply_attribute(classes, &self.negative), apply_attribute(classes, &self.positive));
    }

    pub fn information_gain(&self, classes: &HashMap<Class, BddParams>) -> f64 {
        let original_entropy = entropy(classes);
        let (left, right) = self.restrict(classes);
        //println!("L: {}, R: {}", left.len(), right.len());
        return original_entropy - (0.5 * entropy(&left) + 0.5 * entropy(&right));
    }

}

pub fn make_decision_tree(network: &BooleanNetwork, classes: &HashMap<Class, BddParams>) {
    for v in network.graph().variable_ids() {
        if network.get_update_function(v).is_some() {
            panic!("Only fully parametrised networks are supported at the moment.")
        }

        for r in network.graph().regulators(v) {
            let reg = network.graph().find_regulation(r, v).unwrap();
            if reg.get_monotonicity().is_none() {
                //panic!("Regulation with unspecified monotonicity found.")
            }
            if !reg.is_observable() {
                //panic!("Non-observable regulation found.")
            }
        }
    }

    let encoder = BddParameterEncoder::new(network);
    let all = classes.iter().fold(BddParams::from(encoder.bdd_variables.mk_false()), |a, (_, b)| a.union(b));
    let attributes = make_attributes(network, &encoder, &all);

    let mut tree = BDT::new(classes.clone(), attributes);
    tree.learn_tree(10);
    tree.dump_dot();

    /*println!("digraph G {{");
    println!("init__ [label=\"\", style=invis, height=0, width=0];");
    println!("init__ -> 0;");
    let mut node_id = 0;
    let encoder = BddParameterEncoder::new(network);
    let all = classes.iter().fold(BddParams::from(encoder.bdd_variables.mk_false()), |a, (_, b)| a.union(b));
    let attributes = make_attributes(network, &encoder, &all);

    /*
    // Set fixed A=1
    let A = network.graph().find_variable("A").unwrap();
    let a_table = encoder.implicit_function_table(A);
    assert_eq!(a_table.len(), 1);
    let a_p = encoder.get_implicit_for_table(&a_table[0]);
    let mut classes = classes.clone();
    for (_, v) in classes.iter_mut() {
        *v = v.intersect(&a_p);
    }*/

    let mut remaining = classes.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
    let (a, b, c) = learn(network, &encoder, &mut node_id, &attributes, &classes, &mut remaining, None);
    println!("Classified: {}; Scrap: {}; Total: {}", a, b, c);
    println!("}}");*/
}

const CUT_OFF: bool = false;

fn learn(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    node_id: &mut usize,
    attributes: &Vec<Attribute>,
    classes: &HashMap<Class, BddParams>,
    remaining: &mut f64,
    prefer_leaf: Option<Class>
) -> (f64, f64, usize) {
    if classes.len() == 0 {
        panic!("This should not happen")
    } else if classes.len() == 1 {
        let (class, params) = classes.iter().next().unwrap();
        let c = params.cardinality();
        *remaining -= c;
        //println!("Remaining: {}; Classified: {}", remaining, c);
        println!("{}[label=\"{}({})\"];", node_id, format!("{}", class).replace("\"", ""), params.cardinality());
        return (c, 0.0, 1);
    }/* else if classes.len() == 2 {
        let mut i = classes.iter();
        let (c1, p2) = i.next().unwrap();
        let (c2, p1) = i.next().unwrap();
        let c = p1.cardinality() + p2.cardinality();
        *remaining -= c;
        //println!("Remaining: {}; Classified: {}", remaining, c);
        println!("{}[label=\"{}({}) and {}({})\"];", node_id, format!("{}", c1).replace("\"", ""), p1.cardinality(), format!("{}", c2).replace("\"", ""), p2.cardinality());
        return (c, 0.0, 1);
    }*/
    if CUT_OFF {
        let cardinality: Vec<f64> = classes.iter().map(|(c, p)| p.cardinality()).collect();
        let total = cardinality.iter().fold(0.0, |a, b| a + *b);
        for c in cardinality {
            if c > 0.8 * total {
                *remaining -= total;
                println!("Remaining: {}; Classified: {}", remaining, c);
                return (total, 0.0, 1);
            }
        }
    }
    let original_entropy = entropy(classes);
    let mut max_gain = f64::NEG_INFINITY;
    let mut max_attribute: Option<Attribute> = None;
    let retained_attributes: Vec<Attribute> = attributes.iter()
        .cloned()
        .filter(|attr| {
            let positive_dataset = apply_attribute(classes, &attr.positive);
            let negative_dataset = apply_attribute(classes, &attr.negative);
            let gain = original_entropy
                - (0.5 * entropy(&positive_dataset) + 0.5 * entropy(&negative_dataset));
            /*let gain = if positive_dataset.len() == 1 {
                // prefer fully classified datasets
                let g = positive_dataset.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
                if let Some(preferred) = &prefer_leaf {
                    if positive_dataset.get(&preferred).is_some() {
                        f64::INFINITY
                    } else {
                        g
                    }
                } else {
                    g
                }
            } else if negative_dataset.len() == 1 {
                let g = negative_dataset.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
                if let Some(preferred) = &prefer_leaf {
                    if negative_dataset.get(&preferred).is_some() {
                        f64::INFINITY
                    } else {
                        g
                    }
                } else {
                    g
                }
            } else {
                original_entropy
                    - (0.5 * entropy(&positive_dataset) + 0.5 * entropy(&negative_dataset))
            };*/
            if gain > max_gain {
                max_gain = gain;
                max_attribute = Some(attr.clone());
            }
            let all_c = classes.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
            let p_c = positive_dataset.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
            let n_c = negative_dataset.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
            //println!("{} gain {} // {}({}%) | {}({}%)", attr.name, gain, positive_dataset.len(), ((p_c/all_c) * 100.0) as i32, negative_dataset.len(), ((n_c/all_c) * 100.0) as i32);
            gain > f64::NEG_INFINITY
        }).collect();

    if let Some(attr) = max_attribute {
        //println!("Selected {}", attr.name);
        let my_node_id = *node_id;
        println!("{}[label=\"{}\"];", my_node_id, attr.name);
        *node_id += 1;
        println!("{} -> {} [style=filled];", my_node_id, node_id);
        let positive_dataset = apply_attribute(classes, &attr.positive);
        let negative_dataset = apply_attribute(classes, &attr.negative);
        let prefer = if positive_dataset.len() == 1 {
            Some(positive_dataset.iter().next().unwrap().0.clone())
        } else if negative_dataset.len() == 1 {
            Some(negative_dataset.iter().next().unwrap().0.clone())
        } else { None };
        let (a1, b1, c1) = learn(network, encoder, node_id, &retained_attributes, &positive_dataset, remaining, None);
        *node_id += 1;
        println!("{} -> {} [style=dotted];", my_node_id, node_id);
        let (a2, b2, c2) = learn(network, encoder, node_id, &retained_attributes, &negative_dataset, remaining, None);
        return (a1 + a2, b1 + b2, c1 + c2 + 1);
    } else {
        /*println!("Cannot learn more! Problematic witness:");
        let (_, params) = classes.iter().next().unwrap();
        println!("{}", network.make_witness(params, encoder));
        panic!("");*/
        let scrap = classes.iter().fold(0.0, |a, (_, p)| a + p.cardinality());
        *remaining -= scrap;
        println!("Remaining: {}; Scrap: {}", remaining, scrap);
        return (0.0, scrap, 1);
    }
}

fn make_attributes(network: &BooleanNetwork, encoder: &BddParameterEncoder, all: &BddParams) -> Vec<Attribute> {
    let mut result = Vec::new();
    let graph = network.graph();
    /*for target in network.graph().variable_ids() {
        let regulators = network.graph().regulators(target);
        for a in regulators.iter() {
            for b in regulators.iter() {
                if a == b {
                    continue;
                }
                let r_a = graph.find_regulation(*a, target).unwrap();
                let r_b = graph.find_regulation(*b, target).unwrap();
                result.push(make_enables(network, &encoder, target, *a, *b));
                result.push(make_disables(network, &encoder, target, *a, *b));
                //result.push(make_eq(network, encoder, target, *a, *b));
                //result.push(make_xor(network, encoder, target, *a, *b));
                // if they also have equal monotonicity, we can consider cooperation
                // a < b ensures we only build one attribute for each pair, since coop(a,b) = coop(b,a)
                if r_a.get_monotonicity() == r_b.get_monotonicity() && a < b {
                    result.push(make_cooperation(network, &encoder, target, *a, *b));
                    result.push(make_anti_cooperation(network, &encoder, target, *a, *b));
                }
                if a < b {
                    // these are also symmetric in a-b, so no need to create them twice
                    for c in regulators.iter() {
                        if c == a || c == b {
                            continue;
                        }
                        result.push(make_enables2(network, encoder, target, (*a, *b), *c));
                        result.push(make_disables2(network, encoder, target, (*a, *b), *c));
                        if a < c && b < c {
                            for d in regulators.iter() {
                                if d == a || d == b || d == c {
                                    continue;
                                }
                                result.push(make_enables3(network, encoder, target, (*a, *b, *c), *d));
                                result.push(make_disables3(network, encoder, target, (*a, *b, *c), *d));
                            }
                        }
                    }
                }
            }
        }
        /*for entry in encoder.implicit_function_table(target) {
            let var = encoder.get_implicit_var_for_table(&entry);
            result.push(Attribute {
                name: format!("entry {:?} in {}", var, network.graph().get_variable(target)),
                positive: BddParams::from(encoder.bdd_variables.mk_var(var)),
                negative: BddParams::from(encoder.bdd_variables.mk_not_var(var))
            })
        }*/
    }*/

/*
    let dna = network.graph().find_variable("DNA").unwrap();
    let p53 = network.graph().find_variable("P53").unwrap();
    let m2n = network.graph().find_variable("M2N").unwrap();
    let dna_p53_in_dna = make_enables(network, encoder, dna, dna, p53);
    let dna_p53_in_m2n = make_enables(network, encoder, m2n, dna, p53);
    let positive = dna_p53_in_dna.positive.intersect(&dna_p53_in_m2n.positive);
    result.push(Attribute {
        name: format!("DNA enables P53"),
        negative: BddParams::from(positive.clone().into_bdd().not()),
        positive,
    });

    let dna_p53_in_dna = make_disables(network, encoder, dna, dna, p53);
    let dna_p53_in_m2n = make_disables(network, encoder, m2n, dna, p53);
    let positive = dna_p53_in_dna.positive.intersect(&dna_p53_in_m2n.positive);
    result.push(Attribute {
        name: format!("DNA disables P53"),
        negative: BddParams::from(positive.clone().into_bdd().not()),
        positive,
    });
*/
/*
    let A = network.graph().find_variable("A").unwrap();
    let B = network.graph().find_variable("B").unwrap();
    let C = network.graph().find_variable("C").unwrap();

    result.push(make_conditional_observability(&network, &encoder, C, A, vec![(B, true)]));
    result.push(make_conditional_observability(&network, &encoder, C, A, vec![(B, false)]));
    result.push(make_conditional_observability(&network, &encoder, C, B, vec![(A, true)]));
    result.push(make_conditional_observability(&network, &encoder, C, B, vec![(A, false)]));

    result.push(make_conditional_observability(&network, &encoder, A, A, vec![(B, true)]));
    result.push(make_conditional_observability(&network, &encoder, A, A, vec![(B, false)]));
    result.push(make_conditional_observability(&network, &encoder, A, B, vec![(A, true)]));
    result.push(make_conditional_observability(&network, &encoder, A, B, vec![(A, false)]));

    result.push(make_conditional_non_observability(&network, &encoder, C, A, vec![(B, true)]));
    result.push(make_conditional_non_observability(&network, &encoder, C, A, vec![(B, false)]));
    result.push(make_conditional_non_observability(&network, &encoder, C, B, vec![(A, true)]));
    result.push(make_conditional_non_observability(&network, &encoder, C, B, vec![(A, false)]));

    result.push(make_conditional_non_observability(&network, &encoder, A, A, vec![(B, true)]));
    result.push(make_conditional_non_observability(&network, &encoder, A, A, vec![(B, false)]));
    result.push(make_conditional_non_observability(&network, &encoder, A, B, vec![(A, true)]));
    result.push(make_conditional_non_observability(&network, &encoder, A, B, vec![(A, false)]));

    result.push(make_activation(&network, &encoder, A, A));
    result.push(make_inhibition(&network, &encoder, A, A));
    result.push(make_activation(&network, &encoder, C, A));
    result.push(make_inhibition(&network, &encoder, C, A));
    result.push(make_activation(&network, &encoder, A, B));
    result.push(make_inhibition(&network, &encoder, A, B));
    result.push(make_inhibition(&network, &encoder, C, B));
    result.push(make_activation(&network, &encoder, C, B));
    result.push(make_activation(&network, &encoder, B, C));
    result.push(make_inhibition(&network, &encoder, B, C));

    result.push(iff(
        "B=1 => (A ->! C) iff (A ->! A)",
        make_conditional_observability(&network, &encoder, C, A, vec![(B, true)]),
        make_conditional_observability(&network, &encoder, A, A, vec![(B, true)]),
    ));

    result.push(both(
        "B=1 => A observable in {C and A}",
        make_conditional_observability(&network, &encoder, C, A, vec![(B, true)]),
        make_conditional_observability(&network, &encoder, A, A, vec![(B, true)]),
    ));

    result.push(both(
        "B=1 => A non-observable in {C and A}",
        make_conditional_non_observability(&network, &encoder, C, A, vec![(B, true)]),
        make_conditional_non_observability(&network, &encoder, A, A, vec![(B, true)]),
    ));

 */

/*
    let a = make_disables(network, encoder, m2n, dna, p53);
    let b = make_disables(network, encoder, m2n, p53, dna);
    let when = a.positive.intersect(&b.positive).intersect(all);
    println!("{}", network.make_witness(&when, encoder));*/

    let dna = network.graph().find_variable("DNA").unwrap();
    let p53 = network.graph().find_variable("P53").unwrap();
    let m2n = network.graph().find_variable("M2N").unwrap();
    let m2c = network.graph().find_variable("M2C").unwrap();

/*
    for v in network.graph().variable_ids() {
        for (i, entry) in encoder.implicit_function_table(v).iter().enumerate() {
            result.push(Attribute {
                name: format!("{:?}: {:?}", v, i),
                positive: encoder.get_implicit_for_table(&entry),
                negative: all.minus(&encoder.get_implicit_for_table(&entry)),
            })
        }
    }
*/

    result.push(make_conditional_observability(&network, &encoder,
                                               dna, dna, Vec::new()
    ));

    // P53 enables/disables DNA in DNA
    result.push(make_enables(&network, &encoder, dna, p53, dna));
    result.push(make_disables(&network, &encoder, dna, p53, dna));
    // DNA enables/disables P53 in DNA
    result.push(make_enables(&network, &encoder, dna, dna, p53));
    result.push(make_disables(&network, &encoder, dna, dna, p53));

    // P53 enables/disables DNA/M2C in M2N
    result.push(make_enables(&network, &encoder, m2n, p53, dna));
    result.push(make_disables(&network, &encoder, m2n, p53, dna));
    result.push(make_enables(&network, &encoder, m2n, p53, m2c));
    result.push(make_disables(&network, &encoder, m2n, p53, m2c));
    // DNA ...
    result.push(make_enables(&network, &encoder, m2n, dna, p53));
    result.push(make_disables(&network, &encoder, m2n, dna, p53));
    result.push(make_enables(&network, &encoder, m2n, dna, m2c));
    result.push(make_disables(&network, &encoder, m2n, dna, m2c));
    // M2C ...
    result.push(make_enables(&network, &encoder, m2n, m2c, dna));
    result.push(make_disables(&network, &encoder, m2n, m2c, dna));
    result.push(make_enables(&network, &encoder, m2n, m2c, p53));
    result.push(make_disables(&network, &encoder, m2n, m2c, p53));

    result.push(make_cooperation(&network, &encoder, dna, dna, p53));
    result.push(make_cooperation(&network, &encoder, m2n, dna, p53));
    result.push(make_cooperation(&network, &encoder, m2n, dna, m2c));
    result.push(make_cooperation(&network, &encoder, m2n, m2c, p53));

    result.push(both(
        "DNA observable in M2N regardless of P53",
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(p53, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(p53, false)]
        )
    ));
    result.push(both(
        "DNA observable in M2N regardless of M2C",
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(m2c, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(m2c, false)]
        )
    ));
    result.push(both(
        "P53 observable in M2N regardless of M2C",
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(m2c, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(m2c, false)]
        )
    ));
    result.push(both(
        "P53 observable in M2N regardless of DNA",
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(dna, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(dna, false)]
        )
    ));
    result.push(both(
        "M2C observable in M2N regardless of P53",
        make_conditional_observability(
            &network, &encoder, m2n, m2c, vec![(p53, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, m2c, vec![(p53, false)]
        )
    ));
    result.push(both(
        "M2C observable in M2N regardless of DNA",
        make_conditional_observability(
            &network, &encoder, m2n, m2c, vec![(dna, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, m2c, vec![(dna, false)]
        )
    ));

    result.push(both(
        "P53 observable in DNA regardless of DNA",
        make_conditional_observability(
            &network, &encoder, dna, p53, vec![(dna, true)]
        ),
        make_conditional_observability(
            &network, &encoder, dna, p53, vec![(dna, false)]
        )
    ));
    result.push(both(
        "DNA observable in DNA regardless of P53",
        make_conditional_observability(
            &network, &encoder, dna, dna, vec![(p53, true)]
        ),
        make_conditional_observability(
            &network, &encoder, dna, dna, vec![(p53, false)]
        )
    ));


    // Global observability of P53 when DNA
    result.push(both(
        "P53 observable when DNA=1",
        make_conditional_observability(
            &network, &encoder, dna, p53, vec![(dna, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(dna, true)]
        )
    ));
    result.push(both(
        "P53 observable when DNA=0",
        make_conditional_observability(
            &network, &encoder, dna, p53, vec![(dna, false)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(dna, false)]
        )
    ));


    result.push(iff(
        "(P53 observable in DNA) iff (P53 observable in M2N) when DNA=1",
        make_conditional_observability(
            &network, &encoder, dna, p53, vec![(dna, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(dna, true)]
        )
    ));
    result.push(iff(
        "(P53 observable in DNA) iff (P53 observable in M2N) when DNA=0",
        make_conditional_observability(
            &network, &encoder, dna, p53, vec![(dna, false)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, p53, vec![(dna, false)]
        )
    ));

    // Global observability of DNA when P53
    /*result.push(both(
        "DNA observable when (P53, 1)",
        make_conditional_observability(
            &network, &encoder, dna, dna, vec![(p53, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(p53, true)]
        )
    ));
    result.push(both(
        "DNA observable when (P53, 0)",
        make_conditional_observability(
            &network, &encoder, dna, dna, vec![(p53, false)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(p53, false)]
        )
    ));*/

/*    // DNA Observable when regardless of P53
    result.push(both("DNA observable regardless of P53", both(
        "DNA observable when (P53, 1)",
        make_conditional_observability(
            &network, &encoder, dna, dna, vec![(p53, true)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(p53, true)]
        )
    ),both(
        "DNA observable when (P53, 0)",
        make_conditional_observability(
            &network, &encoder, dna, dna, vec![(p53, false)]
        ),
        make_conditional_observability(
            &network, &encoder, m2n, dna, vec![(p53, false)]
        )
    )));*/



    /*result.push(iff(
        "P53=1 => (DNA ->! DNA) iff (DNA ->! M2N)",
        make_conditional_observability(&network, &encoder, dna, dna, vec![(p53, true)]),
        make_conditional_observability(&network, &encoder, m2n, dna, vec![(p53, true)]),
    ));

    result.push(iff(
        "P53=0 => (DNA ->! DNA) iff (DNA ->! M2N)",
        make_conditional_observability(&network, &encoder, dna, dna, vec![(p53, false)]),
        make_conditional_observability(&network, &encoder, m2n, dna, vec![(p53, false)]),
    ));*/

    // DNA observability
    result.push(make_conditional_observability(
        &network, &encoder,dna, dna, vec![(p53, true)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, dna, dna, vec![(p53, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, dna, p53, vec![(dna, true)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, dna, p53, vec![(dna, false)]
    ));

    // M2N Observability
    // DNA obs. conditional on one other
    result.push(make_conditional_observability(
        &network, &encoder, m2n, dna, vec![(p53, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, dna, vec![(p53, true)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, dna, vec![(m2c, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, dna, vec![(m2c, true)]
    ));

    // P53 obs. conditional on one other
    result.push(make_conditional_observability(
        &network, &encoder, m2n, p53, vec![(dna, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, p53, vec![(dna, true)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, p53, vec![(m2c, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, p53, vec![(m2c, true)]
    ));

    // M2C obs. conditional on one other
    result.push(make_conditional_observability(
        &network, &encoder, m2n, m2c, vec![(p53, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, m2c, vec![(p53, true)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, m2c, vec![(dna, false)]
    ));
    result.push(make_conditional_observability(
        &network, &encoder, m2n, m2c, vec![(dna, true)]
    ));

    result.push(make_conditional_enables(&network, &encoder, m2n, m2c, p53, vec![(dna, true)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, m2c, p53, vec![(dna, false)]));

    result.push(make_conditional_enables(&network, &encoder, m2n, m2c, dna, vec![(p53, true)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, m2c, dna, vec![(p53, false)]));

    result.push(make_conditional_enables(&network, &encoder, m2n, p53, m2c, vec![(dna, true)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, p53, m2c, vec![(dna, false)]));

    result.push(make_conditional_enables(&network, &encoder, m2n, p53, dna, vec![(m2c, true)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, p53, dna, vec![(m2c, false)]));

    result.push(make_conditional_enables(&network, &encoder, m2n, dna, p53, vec![(m2c, true)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, dna, p53, vec![(m2c, false)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, dna, m2c, vec![(p53, true)]));
    result.push(make_conditional_enables(&network, &encoder, m2n, dna, m2c, vec![(p53, false)]));

/*
    result.push(make_eq(&network, &encoder, m2n, dna, m2c));
    result.push(make_eq(&network, &encoder, m2n, dna, p53));
    result.push(make_eq(&network, &encoder, m2n, p53, m2c));
    result.push(make_xor(&network, &encoder, m2n, dna, m2c));
    result.push(make_xor(&network, &encoder, m2n, dna, p53));
    result.push(make_xor(&network, &encoder, m2n, p53, m2c));
 */

    /*let dna_observable_in_dna = make_conditional_observability(
        &network, &encoder, dna, dna, vec![]
    );
    let dna_observable_in_m2n = make_conditional_observability(
        &network, &encoder, m2n, dna, vec![]
    );
    let neither = dna_observable_in_dna.negative.into_bdd().and(&dna_observable_in_m2n.negative.into_bdd());
    result.push(Attribute {
        name: "DNA not observable".to_string(),
        negative: BddParams::from(neither.not()),
        positive: BddParams::from(neither),
    });*/

    return result;
}

fn both(name: &str, a: Attribute, b: Attribute) -> Attribute {
    let valid = a.positive.into_bdd().and(&b.positive.into_bdd());
    return Attribute {
        name: name.to_string(),
        negative: BddParams::from(valid.not()),
        positive: BddParams::from(valid)
    }
}

fn iff(name: &str, a: Attribute, b: Attribute) -> Attribute {
    let valid = a.positive.into_bdd().iff(&b.positive.into_bdd());
    return Attribute {
        name: name.to_string(),
        negative: BddParams::from(valid.not()),
        positive: BddParams::from(valid)
    }
}

fn make_conditional_enables(network: &BooleanNetwork, encoder: &BddParameterEncoder, target: VariableId, a: VariableId, b: VariableId, conditionals: Vec<(VariableId, bool)>) -> Attribute {
    let table: Vec<FunctionTableEntry> = encoder.implicit_function_table(target)
        .into_iter()
        .filter(|e| {
            let cond = conditionals.iter().all(|(c, v)| e.get_value(*c) == *v);
            let a_zero = !e.get_value(a);
            let b_zero = !e.get_value(b);
            cond && a_zero && b_zero
        })
        .collect();
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        //if entry.get_value(a) == false && entry.get_value(b) == false {
            let zero_zero = encoder.get_implicit_var_for_table(&entry);
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let zero_zero = encoder.bdd_variables.mk_var(zero_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & (zero_zero <=> zero_one)];
        //}
    }
    return Attribute {
        name: format!(
            "({} enables {}) in {} when {:?}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target),
            conditionals
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_conditional_observability(network: &BooleanNetwork, encoder: &BddParameterEncoder, target: VariableId, regulator: VariableId, conditionals: Vec<(VariableId, bool)>) -> Attribute {
    let table: Vec<FunctionTableEntry> = encoder.implicit_function_table(target)
        .into_iter()
        .filter(|e| {
            let cond = conditionals.iter().all(|(c, v)| e.get_value(*c) == *v);
            let reg_zero = !e.get_value(regulator);
            cond && reg_zero
        })
        .collect();
    let mut params = encoder.bdd_variables.mk_false();
    for entry in table {
        // clause: (regulator: 0, conditionals: 1) != (regulator: 1, conditionals: 1)
        let inactive = encoder.get_implicit_var_for_table(&entry);
        let inactive = encoder.bdd_variables.mk_var(inactive);
        let active = encoder.get_implicit_var_for_table(&entry.flip_value(regulator));
        let active = encoder.bdd_variables.mk_var(active);
        params = bdd!(params | (!(active <=> inactive)));
    }
    return Attribute {
        name: format!(
            "{} observable in {} when {:?}",
            network.graph().get_variable(regulator),
            network.graph().get_variable(target),
            conditionals//.iter().map(|(i, v)| (format!("({}, {})", network.graph().get_variable(*i).get_name(), *v))).collect::<Vec<String>>()
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    }
}

fn make_conditional_non_observability(network: &BooleanNetwork, encoder: &BddParameterEncoder, target: VariableId, regulator: VariableId, conditionals: Vec<(VariableId, bool)>) -> Attribute {
    let table: Vec<FunctionTableEntry> = encoder.implicit_function_table(target)
        .into_iter()
        .filter(|e| {
            let cond = conditionals.iter().all(|(c, v)| e.get_value(*c) == *v);
            let reg_zero = !e.get_value(regulator);
            cond && reg_zero
        })
        .collect();
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        // clause: (regulator: 0, conditionals: 1) != (regulator: 1, conditionals: 1)
        let inactive = encoder.get_implicit_var_for_table(&entry);
        let inactive = encoder.bdd_variables.mk_var(inactive);
        let active = encoder.get_implicit_var_for_table(&entry.flip_value(regulator));
        let active = encoder.bdd_variables.mk_var(active);
        params = bdd!(params & ((active <=> inactive)));
    }
    return Attribute {
        name: format!(
            "{} non-observable in {} when {:?}",
            network.graph().get_variable(regulator),
            network.graph().get_variable(target),
            conditionals//.iter().map(|(i, v)| (format!("({}, {})", network.graph().get_variable(*i).get_name(), *v))).collect::<Vec<String>>()
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    }
}

/// When in cooperation, variables have effect only when active together, meaning
/// (A, B) ... f(0,0) <=> f(1,0) <=> f(0,1)
fn make_cooperation(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == false && entry.get_value(b) == false {
            let zero_zero = encoder.get_implicit_var_for_table(&entry);
            let one_zero = encoder.get_implicit_var_for_table(&entry.flip_value(a));
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let zero_zero = encoder.bdd_variables.mk_var(zero_zero);
            let one_zero = encoder.bdd_variables.mk_var(one_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & ((zero_zero <=> one_zero) & (zero_zero <=> zero_one))];
        }
    }
    return Attribute {
        name: format!(
            "cooperation({}, {}) in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_anti_cooperation(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == true && entry.get_value(b) == true {
            let one_one = encoder.get_implicit_var_for_table(&entry);
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(a));
            let one_zero = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let one_one = encoder.bdd_variables.mk_var(one_one);
            let one_zero = encoder.bdd_variables.mk_var(one_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & ((one_one <=> one_zero) & (one_one <=> zero_one))];
        }
    }
    return Attribute {
        name: format!(
            "anti-cooperation({}, {}) in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

/// When in A disables B, it means B can't have effect when A is active.
/// (A, B) ... f(1,0) <=> f(1,1)
fn make_disables(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == true && entry.get_value(b) == false {
            let one_zero = encoder.get_implicit_var_for_table(&entry);
            let one_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let one_zero = encoder.bdd_variables.mk_var(one_zero);
            let one_one = encoder.bdd_variables.mk_var(one_one);
            params = bdd![params & (one_zero <=> one_one)];
        }
    }
    return Attribute {
        name: format!(
            "({} disables {}) in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_eq(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == false && entry.get_value(b) == false {
            let zero_zero = encoder.get_implicit_var_for_table(&entry);
            let one_one = encoder.get_implicit_var_for_table(&entry.flip_value(b).flip_value(a));
            let zero_zero = encoder.bdd_variables.mk_var(zero_zero);
            let one_one = encoder.bdd_variables.mk_var(one_one);
            params = bdd![params & (zero_zero <=> one_one)];
        }
    }
    return Attribute {
        name: format!(
            "({} EQ {}) in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}


fn make_xor(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == true && entry.get_value(b) == false {
            let one_zero = encoder.get_implicit_var_for_table(&entry);
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(b).flip_value(a));
            let one_zero = encoder.bdd_variables.mk_var(one_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & (one_zero <=> zero_one)];
        }
    }
    return Attribute {
        name: format!(
            "({} XOR {}) in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

/// When A enables B, it means B can't have effect when A is not active.
/// (A, B) ... f(0,0) <=> f(0,1)
fn make_enables(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == false && entry.get_value(b) == false {
            let zero_zero = encoder.get_implicit_var_for_table(&entry);
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let zero_zero = encoder.bdd_variables.mk_var(zero_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & (zero_zero <=> zero_one)];
        }
    }
    return Attribute {
        name: format!(
            "({} enables {}) in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_inhibition(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == false {
            let zero = encoder.get_implicit_var_for_table(&entry);
            let one = encoder.get_implicit_var_for_table(&entry.flip_value(a));
            let zero = encoder.bdd_variables.mk_var(zero);
            let one = encoder.bdd_variables.mk_var(one);
            params = bdd![params & (one => zero)];
        }
    }
    return Attribute {
        name: format!(
            "{} is inhibition in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_activation(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    a: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a) == false {
            let zero = encoder.get_implicit_var_for_table(&entry);
            let one = encoder.get_implicit_var_for_table(&entry.flip_value(a));
            let zero = encoder.bdd_variables.mk_var(zero);
            let one = encoder.bdd_variables.mk_var(one);
            params = bdd![params & (zero => one)];
        }
    }
    return Attribute {
        name: format!(
            "{} is activation in {}",
            network.graph().get_variable(a),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}


/// When (A1,A2) enables B, it means B can't have effect unless A1 or A2 is active.
fn make_enables2(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    (a1, a2): (VariableId, VariableId),
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a1) == false && entry.get_value(a2) == false && entry.get_value(b) == false {
            let zero_zero = encoder.get_implicit_var_for_table(&entry);
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let zero_zero = encoder.bdd_variables.mk_var(zero_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & (zero_zero <=> zero_one)];
        }
    }
    return Attribute {
        name: format!(
            "(({} or {}) enables {}) in {}",
            network.graph().get_variable(a1),
            network.graph().get_variable(a2),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

/// When in (A1, A2) disables B, it means B can't have effect when A1 and A2 is active.
/// (A, B) ... f(1,0) <=> f(1,1)
fn make_disables2(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    (a1, a2): (VariableId, VariableId),
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a1) == true && entry.get_value(a2) == true && entry.get_value(b) == false {
            let one_zero = encoder.get_implicit_var_for_table(&entry);
            let one_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let one_zero = encoder.bdd_variables.mk_var(one_zero);
            let one_one = encoder.bdd_variables.mk_var(one_one);
            params = bdd![params & (one_zero <=> one_one)];
        }
    }
    return Attribute {
        name: format!(
            "(({} and {}) disables {}) in {}",
            network.graph().get_variable(a1),
            network.graph().get_variable(a2),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_disables3(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    (a1, a2, a3): (VariableId, VariableId, VariableId),
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a1) == true && entry.get_value(a2) == true && entry.get_value(a3) == true && entry.get_value(b) == false {
            let one_zero = encoder.get_implicit_var_for_table(&entry);
            let one_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let one_zero = encoder.bdd_variables.mk_var(one_zero);
            let one_one = encoder.bdd_variables.mk_var(one_one);
            params = bdd![params & (one_zero <=> one_one)];
        }
    }
    return Attribute {
        name: format!(
            "(({} and {} and {}) disables {}) in {}",
            network.graph().get_variable(a1),
            network.graph().get_variable(a2),
            network.graph().get_variable(a3),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}

fn make_enables3(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    (a1, a2, a3): (VariableId, VariableId, VariableId),
    b: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(a1) == false && entry.get_value(a2) == false && entry.get_value(a3) == false && entry.get_value(b) == false {
            let zero_zero = encoder.get_implicit_var_for_table(&entry);
            let zero_one = encoder.get_implicit_var_for_table(&entry.flip_value(b));
            let zero_zero = encoder.bdd_variables.mk_var(zero_zero);
            let zero_one = encoder.bdd_variables.mk_var(zero_one);
            params = bdd![params & (zero_zero <=> zero_one)];
        }
    }
    return Attribute {
        name: format!(
            "(({} or {} or {}) enables {}) in {}",
            network.graph().get_variable(a1),
            network.graph().get_variable(a2),
            network.graph().get_variable(a3),
            network.graph().get_variable(b),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}
