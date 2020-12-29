use crate::scc::Class;
use biodivine_lib_param_bn::symbolic_async_graph::{
    GraphColors, SymbolicAsyncGraph, SymbolicContext,
};
use biodivine_lib_param_bn::BooleanNetwork;
use biodivine_lib_std::param_graph::Params;
use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::iter::Map;
use std::ops::Range;

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
        classes: HashMap<Class, GraphColors>,
    },
    Unprocessed {
        classes: HashMap<Class, GraphColors>,
    },
}

/// An identifier of a BDT node. These are used to quickly refer to parts of a BDT, for example
/// from GUI.
///
/// I might want to delete a node - to avoid specifying a full path from root to the deleted node,
/// I can use the ID which will automatically "jump" to the correct position in the tree.
#[derive(Debug, Clone, Copy)]
pub struct BDTNodeId(usize);

/// An attribute id is used to identify a specific attribute used in a decision tree.
///
/// These are bound to a specific BDT, but note that not all attributes have to be applicable
/// to all BDT nodes (or, more specifically, they are applicable but have no effect).
#[derive(Debug, Clone, Copy)]
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

impl BDT {
    pub fn learn_tree(&mut self, max_depth: usize) {
        let attr: Vec<usize> = (0..(self.attributes.len())).collect();
        return self.learn_tree_recursive(0, &attr, 0, max_depth);
    }

    fn learn_tree_recursive(
        &mut self,
        node: usize,
        attr: &Vec<usize>,
        depth: usize,
        max_depth: usize,
    ) {
        if depth >= max_depth {
            return;
        }
        match &self.storage[&node] {
            BDTNode::Leaf { .. } => return, // already processed, skip
            BDTNode::Unprocessed { classes } => {
                // Find best attribute and continue.
                let mut continue_with = Vec::new();
                let mut max: Option<(usize, f64)> = None;
                for a in attr.iter() {
                    let attribute = &self.attributes[*a];
                    let gain = attribute.information_gain(classes);
                    println!("{}: Gain {} from {}.", a, gain, attribute.name);
                    if gain > f64::NEG_INFINITY {
                        continue_with.push(*a);
                        if let Some((_, current_gain)) = max {
                            if gain > current_gain {
                                max = Some((*a, gain));
                            }
                        } else {
                            max = Some((*a, gain));
                        }
                    }
                }
                if let Some((max, max_gain)) = max {
                    //let max = if max == 29 { /*27*/ /*31*/ /*44*/ 4 } else { max };
                    //let max = if max == 11 { 27 } else { max };
                    /*let max = if max == 36 && depth == 1 { /*6*/ 48 } else { max };
                    let max = if max == 4 && depth == 2 { 8 } else { max };
                    let max = if max == 29 && depth == 2 { 36 } else { max };
                    let max = if max == 4 && depth == 3 { 7 } else { max };
                    let max = if max == 6 && depth == 0 { 49 } else { max };*/
                    /*FINAL TREE: let max = if max == 0 && depth == 0 { 2 } else { max };
                    let max = if max == 9 && depth == 1 { 52 } else { max };*/
                    //let max = if (max == 7 || max == 8) && depth == 0 { /*36*/ 51 } else { max };
                    //let max = if max == 4 && depth == 1 { 51 } else { max };
                    //let max = if (max == 50 || max == 49) && depth == 0 { 53 } else { max };
                    let max = if depth == 0 { 50 } else { max }; // root: is essential?
                    let max = if depth == 1 && max == 22 { 60 } else { max }; // if not essential, is it true?
                    println!(
                        "Select attr: {}: {} with gain {}",
                        max, self.attributes[max].name, max_gain
                    );
                    let (l, r) = self
                        .make_decision(BDTNodeId(node), AttributeId(max))
                        .unwrap();
                    self.learn_tree_recursive(l.0, &continue_with, depth + 1, max_depth);
                    self.learn_tree_recursive(r.0, &continue_with, depth + 1, max_depth);
                } else {
                    panic!("No suitable attribute found!")
                }
            }
            BDTNode::Decision { left, right, .. } => {
                // Processed, but maybe has unprocessed children!
                let (left, right) = (left.0, right.0); // hint for borrow checker to release reference to self
                self.learn_tree_recursive(left, attr, depth + 1, max_depth);
                self.learn_tree_recursive(right, attr, depth + 1, max_depth);
            }
        }
    }
}

/// Restrict given classes using the specified attribute parameters
fn apply_attribute(
    classes: &HashMap<Class, GraphColors>,
    attribute: &GraphColors,
) -> HashMap<Class, GraphColors> {
    let mut result = HashMap::new();
    for (c, p) in classes {
        let new_p = Params::intersect(attribute, p);
        if !new_p.is_empty() {
            result.insert(c.clone(), new_p);
        }
    }
    return result;
}

/// Compute entropy of the behaviour class data set
fn entropy(classes: &HashMap<Class, GraphColors>) -> f64 {
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

#[derive(Clone)]
pub struct Attribute {
    name: String,
    positive: GraphColors,
    negative: GraphColors,
}

pub struct AppliedAttribute {
    attribute: AttributeId,
    left: HashMap<Class, GraphColors>,
    right: HashMap<Class, GraphColors>,
    information_gain: f64,
}

impl Attribute {
    pub fn restrict(
        &self,
        classes: &HashMap<Class, GraphColors>,
    ) -> (HashMap<Class, GraphColors>, HashMap<Class, GraphColors>) {
        return (
            apply_attribute(classes, &self.negative),
            apply_attribute(classes, &self.positive),
        );
    }

    pub fn information_gain(&self, classes: &HashMap<Class, GraphColors>) -> f64 {
        let original_entropy = entropy(classes);
        let (left, right) = self.restrict(classes);
        return original_entropy - (0.5 * entropy(&left) + 0.5 * entropy(&right));
    }
}

fn information_gain(original: f64, left: f64, right: f64) -> f64 {
    original - (0.5 * left + 0.5 * right)
}

/*
   We currently consider the following attributes:
    - general observability/activation/inhibition (if not specified)
    - context aware observability - context are values of

   Note: Observability has been renamed to "X essential in Y", which we currently do not reflect
   in naming of variables and functions, only in the names of the attributes.
*/
pub fn make_network_attributes(network: &BooleanNetwork) -> Vec<Attribute> {
    let ref encoder = SymbolicAsyncGraph::new(network.clone()).unwrap();
    let mut result = Vec::new();
    /*for target in network.graph().variable_ids() {
        for regulator in network.graph().regulators(target) {
            let regulation = network.graph().find_regulation(regulator, target).unwrap();
            if regulation.get_monotonicity() == None {
                result.push(make_activation(network, encoder, target, regulator));
                result.push(make_inhibition(network, encoder, target, regulator));
            }
            for context_variables in context_combinations(network, target, regulator) {
                println!(
                    "Context of {:?} -> {:?} is {:?}",
                    regulator, target, context_variables
                );
                let wildcards = wildcards_for_context(
                    &context_variables,
                    &network.graph().regulators(target),
                    regulator,
                );
                println!("Wildcards are {:?}", wildcards);
                let contexts = conditionals_for_context(context_variables);
                for context in &contexts {
                    println!("Conditionals: {:?}", context);
                    result.push(make_conditional_observability(
                        network, encoder, target, regulator, context,
                    ));
                    //result.push(make_conditional_non_observability(network, encoder, target, regulator, &context));
                }
                for wildcards in wildcard_combinations(&wildcards) {
                    if wildcards.is_empty() {
                        continue;
                    }
                    for context in &contexts {
                        let mut positive = encoder.bdd_variables.mk_true();
                        for wildcard_context in conditionals_for_context(wildcards.clone()) {
                            let mut total_context = context.clone();
                            for i in wildcard_context {
                                total_context.push(i);
                            }
                            let attr = make_conditional_observability(
                                network,
                                encoder,
                                target,
                                regulator,
                                &total_context,
                            );
                            positive = positive.and(&attr.positive.into_bdd());
                        }
                        result.push(Attribute {
                            name: format!(
                                "{} essential in {} for {}.",
                                network.graph().get_variable(regulator),
                                network.graph().get_variable(target),
                                format!(
                                    "{:?} wildcard {:?}",
                                    context
                                        .iter()
                                        .map(|(v, b)| format!(
                                            "{}={}",
                                            network.graph().get_variable(*v),
                                            b
                                        ))
                                        .collect::<Vec<String>>(),
                                    wildcards
                                        .iter()
                                        .map(|v| network
                                            .graph()
                                            .get_variable(*v)
                                            .get_name()
                                            .clone())
                                        .collect::<Vec<String>>()
                                )
                                .replace("\"", "")
                                .as_str(),
                            ),
                            negative: BddParams::from(positive.not()),
                            positive: BddParams::from(positive),
                        })
                    }
                }
            }
        }
    }*/
    
    for v in network.variables() {
        if network.get_update_function(v).is_none() {
            if !network.regulators(v).is_empty() {
                panic!("Unsupported network with non-trivial parameters?");
            } else {
                // There should be exactly one BDD variable corresponding to value of this "parameter"
                let bdd = encoder
                    .symbolic_context()
                    .mk_implicit_function_is_true(v, &vec![]);
                let negative = encoder
                    .unit_colors()
                    .copy(bdd.not())
                    .intersect(encoder.unit_colors());
                let positive = encoder
                    .unit_colors()
                    .copy(bdd)
                    .intersect(encoder.unit_colors());
                result.push(Attribute {
                    name: format!("{}", network.get_variable_name(v)),
                    positive,
                    negative,
                })
            }
        }
    }
    for p in network.parameters() {
        let parameter = network.get_parameter(p);
        if parameter.get_arity() > 0 {
            panic!("Unsupported network with non-trivial parameters?");
        } else {
            // There should be exactly one BDD variable corresponding to value of this "parameter"
            let bdd = encoder
                .symbolic_context()
                .mk_uninterpreted_function_is_true(p, &vec![]);
            let negative = encoder
                .unit_colors()
                .copy(bdd.not())
                .intersect(encoder.unit_colors());
            let positive = encoder
                .unit_colors()
                .copy(bdd)
                .intersect(encoder.unit_colors());
            result.push(Attribute {
                name: format!("{}", parameter.get_name()),
                positive,
                negative,
            })
        }
    }

    return result;
}

/// Find implicit function table entries which satisfy given context conditions
/*fn implicit_function_table<'a, 'b>(
    encoder: &'a BddParameterEncoder,
    target: VariableId,
    conditionals: &'b Vec<(VariableId, bool)>,
) -> Vec<FunctionTableEntry<'a>> {
    let table = encoder.implicit_function_table(target);
    let mut result = Vec::new();
    for entry in table {
        if conditionals.iter().all(|(v, b)| entry.get_value(*v) == *b) {
            result.push(entry);
        }
    }
    return result;
}*/

/*
/// Construct all context variable combinations.
///
/// For example, given regulators [A, B, C, D, E] where C is the significant regulator, the function
/// will produce: [
///     [],
///     [A], [B], [D], [E],
///     [A, B], [A, D], [A, E], [B, D], [B, E], [D, E]
///     [A, B, D], [A, B, E], [A, D, E], [B, D, E]
///     [A, B, D, E]
/// ]
///
fn context_combinations(
    network: &BooleanNetwork,
    target: VariableId,
    regulator: VariableId,
) -> Vec<Vec<VariableId>> {
    let mut result = Vec::new();
    let mut partial_context = Vec::new();
    context_combinations_recursive(
        &mut result,
        &mut partial_context,
        &network.graph().regulators(target),
        regulator,
    );
    return result;
}
 */

/*
fn context_combinations_recursive(
    result: &mut Vec<Vec<VariableId>>,
    partial_context: &mut Vec<VariableId>,
    all_regulators: &Vec<VariableId>,
    regulator: VariableId,
) {
    result.push((*partial_context).clone());
    for candidate in all_regulators {
        let is_valid = partial_context.iter().all(|present| present < candidate);
        if is_valid && *candidate != regulator {
            partial_context.push(*candidate);
            context_combinations_recursive(result, partial_context, all_regulators, regulator);
            partial_context.pop();
        }
    }
}*/

/*
fn wildcard_combinations(wildcards: &Vec<VariableId>) -> Vec<Vec<VariableId>> {
    let mut result = Vec::new();
    let mut partial = Vec::new();
    wildcard_combinations_recursive(&mut result, wildcards, &mut partial);
    return result;
}*/

/*
fn wildcard_combinations_recursive(
    result: &mut Vec<Vec<VariableId>>,
    wildcards: &Vec<VariableId>,
    partial_tuple: &mut Vec<VariableId>,
) {
    result.push((*partial_tuple).clone());
    for candidate in wildcards {
        let is_valid = partial_tuple.iter().all(|present| present < candidate);
        if is_valid {
            partial_tuple.push(*candidate);
            wildcard_combinations_recursive(result, wildcards, partial_tuple);
            partial_tuple.pop();
        }
    }
}*/

/// Produce all conditional assignment of the given context variables.
/*fn conditionals_for_context(context_variables: Vec<VariableId>) -> Vec<Vec<(VariableId, bool)>> {
    let mut result = Vec::new();
    for i in 0..(1 << context_variables.len()) {
        let mut conditionals = Vec::new();
        for i_var in 0..context_variables.len() {
            let expected_value = (i >> i_var) & 1 == 1;
            conditionals.push((context_variables[i_var], expected_value));
        }
        result.push(conditionals);
    }
    return result;
}*/

/*
fn context_get_value(context: &Vec<(VariableId, bool)>, var: VariableId) -> Option<bool> {
    for (v, b) in context {
        if *v == var {
            return Some(*b);
        }
    }
    return None;
}*/

/*
fn context_flip_value(
    context: &Vec<(VariableId, bool)>,
    var: VariableId,
) -> Vec<(VariableId, bool)> {
    let mut result = context.clone();
    for (v, b) in result.iter_mut() {
        if *v == var {
            *b = !*b;
        }
    }
    return result;
}*/
/*
fn wildcards_for_context(
    context_variables: &Vec<VariableId>,
    regulators: &Vec<VariableId>,
    regulator: VariableId,
) -> Vec<VariableId> {
    let mut result = Vec::new();
    for reg in regulators {
        if !context_variables.contains(reg) && *reg != regulator {
            result.push(*reg);
        }
    }
    return result;
}*/
/*
fn make_conditional_observability(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    regulator: VariableId,
    conditionals: &Vec<(VariableId, bool)>,
) -> Attribute {
    let table: Vec<FunctionTableEntry> = implicit_function_table(encoder, target, conditionals);
    let mut params = encoder.bdd_variables.mk_false();
    for entry in table {
        if entry.get_value(regulator) == false {
            // clause: (regulator: 0, conditionals: 1) != (regulator: 1, conditionals: 1)
            let inactive = encoder.get_implicit_var_for_table(&entry);
            let inactive = encoder.bdd_variables.mk_var(inactive);
            let active = encoder.get_implicit_var_for_table(&entry.flip_value(regulator));
            let active = encoder.bdd_variables.mk_var(active);
            params = bdd!(params | (!(active <=> inactive)));
        }
    }
    return Attribute {
        name: format!(
            "{} essential in {} for {}.",
            network.graph().get_variable(regulator),
            network.graph().get_variable(target),
            format!(
                "{:?}",
                conditionals
                    .iter()
                    .map(|(v, b)| format!("{}={}", network.graph().get_variable(*v), b))
                    .collect::<Vec<String>>()
            )
            .replace("\"", ""),
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}*/
/*
fn make_conditional_non_observability(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    regulator: VariableId,
    conditionals: &Vec<(VariableId, bool)>,
) -> Attribute {
    let table: Vec<FunctionTableEntry> = implicit_function_table(encoder, target, conditionals);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(regulator) == false {
            // clause: (regulator: 0, conditionals: 1) != (regulator: 1, conditionals: 1)
            let inactive = encoder.get_implicit_var_for_table(&entry);
            let inactive = encoder.bdd_variables.mk_var(inactive);
            let active = encoder.get_implicit_var_for_table(&entry.flip_value(regulator));
            let active = encoder.bdd_variables.mk_var(active);
            params = bdd!(params & (active <=> inactive));
        }
    }
    return Attribute {
        name: format!(
            "When {}, {} has no effect in {}.",
            format!(
                "{:?}",
                conditionals
                    .iter()
                    .map(|(v, b)| format!("{}={}", network.graph().get_variable(*v), b))
                    .collect::<Vec<String>>()
            )
            .replace("\"", ""),
            network.graph().get_variable(regulator),
            network.graph().get_variable(target),
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}*/

pub fn make_decision_tree(network: &BooleanNetwork, classes: &HashMap<Class, GraphColors>) {
    for v in network.variables() {
        if network.get_update_function(v).is_some() {
            panic!("Only fully parametrised networks are supported at the moment.")
        }

        for r in network.regulators(v) {
            let reg = network.as_graph().find_regulation(r, v).unwrap();
            if reg.get_monotonicity().is_none() {
                //panic!("Regulation with unspecified monotonicity found.")
            }
            if !reg.is_observable() {
                //panic!("Non-observable regulation found.")
            }
        }
    }

    //let encoder = BddParameterEncoder::new(network);
    //let all = classes.iter().fold(BddParams::from(encoder.bdd_variables.mk_false()), |a, (_, b)| a.union(b));
    let attributes = make_network_attributes(network);

    let mut tree = BDT::new(classes.clone(), attributes);
    tree.learn_tree(10);
    println!("{}", tree.to_dot());

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

fn learn(
    network: &BooleanNetwork,
    encoder: &SymbolicContext,
    node_id: &mut usize,
    attributes: &Vec<Attribute>,
    classes: &HashMap<Class, GraphColors>,
    remaining: &mut f64,
    _prefer_leaf: Option<Class>,
) -> (f64, f64, usize) {
    if classes.len() == 0 {
        panic!("This should not happen")
    } else if classes.len() == 1 {
        let (class, params) = classes.iter().next().unwrap();
        let c = params.approx_cardinality();
        *remaining -= c;
        //println!("Remaining: {}; Classified: {}", remaining, c);
        println!(
            "{}[label=\"{}({})\"];",
            node_id,
            format!("{}", class).replace("\"", ""),
            params.approx_cardinality()
        );
        return (c, 0.0, 1);
    } /* else if classes.len() == 2 {
          let mut i = classes.iter();
          let (c1, p2) = i.next().unwrap();
          let (c2, p1) = i.next().unwrap();
          let c = p1.cardinality() + p2.cardinality();
          *remaining -= c;
          //println!("Remaining: {}; Classified: {}", remaining, c);
          println!("{}[label=\"{}({}) and {}({})\"];", node_id, format!("{}", c1).replace("\"", ""), p1.cardinality(), format!("{}", c2).replace("\"", ""), p2.cardinality());
          return (c, 0.0, 1);
      }*/
    let original_entropy = entropy(classes);
    let mut max_gain = f64::NEG_INFINITY;
    let mut max_attribute: Option<Attribute> = None;
    let retained_attributes: Vec<Attribute> = attributes
        .iter()
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
            let _all_c = classes
                .iter()
                .fold(0.0, |a, (_, p)| a + p.approx_cardinality());
            let _p_c = positive_dataset
                .iter()
                .fold(0.0, |a, (_, p)| a + p.approx_cardinality());
            let _n_c = negative_dataset
                .iter()
                .fold(0.0, |a, (_, p)| a + p.approx_cardinality());
            //println!("{} gain {} // {}({}%) | {}({}%)", attr.name, gain, positive_dataset.len(), ((p_c/all_c) * 100.0) as i32, negative_dataset.len(), ((n_c/all_c) * 100.0) as i32);
            gain > f64::NEG_INFINITY
        })
        .collect();

    if let Some(attr) = max_attribute {
        //println!("Selected {}", attr.name);
        let my_node_id = *node_id;
        println!("{}[label=\"{}\"];", my_node_id, attr.name);
        *node_id += 1;
        println!("{} -> {} [style=filled];", my_node_id, node_id);
        let positive_dataset = apply_attribute(classes, &attr.positive);
        let negative_dataset = apply_attribute(classes, &attr.negative);
        let _prefer = if positive_dataset.len() == 1 {
            Some(positive_dataset.iter().next().unwrap().0.clone())
        } else if negative_dataset.len() == 1 {
            Some(negative_dataset.iter().next().unwrap().0.clone())
        } else {
            None
        };
        let (a1, b1, c1) = learn(
            network,
            encoder,
            node_id,
            &retained_attributes,
            &positive_dataset,
            remaining,
            None,
        );
        *node_id += 1;
        println!("{} -> {} [style=dotted];", my_node_id, node_id);
        let (a2, b2, c2) = learn(
            network,
            encoder,
            node_id,
            &retained_attributes,
            &negative_dataset,
            remaining,
            None,
        );
        return (a1 + a2, b1 + b2, c1 + c2 + 1);
    } else {
        /*println!("Cannot learn more! Problematic witness:");
        let (_, params) = classes.iter().next().unwrap();
        println!("{}", network.make_witness(params, encoder));
        panic!("");*/
        let scrap = classes
            .iter()
            .fold(0.0, |a, (_, p)| a + p.approx_cardinality());
        *remaining -= scrap;
        println!("Remaining: {}; Scrap: {}", remaining, scrap);
        return (0.0, scrap, 1);
    }
}

/*
fn make_attributes(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    all: &GraphColors,
) -> Vec<Attribute> {
    let mut result = Vec::new();

    return result;
}
 */
/*
fn both(name: &str, a: Attribute, b: Attribute) -> Attribute {
    let valid = a.positive.into_bdd().and(&b.positive.into_bdd());
    return Attribute {
        name: name.to_string(),
        negative: BddParams::from(valid.not()),
        positive: BddParams::from(valid),
    };
}

fn iff(name: &str, a: Attribute, b: Attribute) -> Attribute {
    let valid = a.positive.into_bdd().iff(&b.positive.into_bdd());
    return Attribute {
        name: name.to_string(),
        negative: BddParams::from(valid.not()),
        positive: BddParams::from(valid),
    };
}
 */

/*
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
*/

/*
fn make_activation(
    network: &BooleanNetwork,
    encoder: &BddParameterEncoder,
    target: VariableId,
    regulator: VariableId,
) -> Attribute {
    let table = encoder.implicit_function_table(target);
    let mut params = encoder.bdd_variables.mk_true();
    for entry in table {
        if entry.get_value(regulator) == false {
            let zero = encoder.get_implicit_var_for_table(&entry);
            let one = encoder.get_implicit_var_for_table(&entry.flip_value(regulator));
            let zero = encoder.bdd_variables.mk_var(zero);
            let one = encoder.bdd_variables.mk_var(one);
            params = bdd![params & (zero => one)];
        }
    }
    return Attribute {
        name: format!(
            "{} is activation in {}",
            network.graph().get_variable(regulator),
            network.graph().get_variable(target)
        ),
        negative: BddParams::from(params.not()),
        positive: BddParams::from(params),
    };
}
*/
