#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

#[macro_use]
extern crate json;

use rocket::http::hyper::header::AccessControlAllowOrigin;
use rocket::http::{ContentType, Header};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use biodivine_aeon_server::scc::{Behaviour, Class, Classifier};
use biodivine_lib_param_bn::{BooleanNetwork, FnUpdate};
use regex::Regex;
use std::convert::TryFrom;

use biodivine_aeon_server::bdt::{AttributeId, Bdt, BdtNodeId};
use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;
use biodivine_aeon_server::scc::algo_stability_analysis::{
    compute_stability, StabilityVector, VariableStability,
};
use biodivine_aeon_server::scc::algo_xie_beerel::xie_beerel_attractors;
use biodivine_aeon_server::util::functional::Functional;
use biodivine_aeon_server::util::index_type::IndexType;
use biodivine_aeon_server::GraphTaskContext;
use biodivine_lib_param_bn::biodivine_std::bitvector::{ArrayBitVector, BitVector};
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColors, SymbolicAsyncGraph};
use json::JsonValue;
use rocket::config::Environment;
use rocket::{Config, Data};
use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Computation keeps all information
struct Computation {
    timestamp: SystemTime,
    input_model: String,    // .aeon string representation of the model
    task: GraphTaskContext, // A task context which keeps track of progress and cancellation.
    graph: Option<Arc<SymbolicAsyncGraph>>, // Model graph - used to create witnesses
    classifier: Option<Arc<Classifier>>, // Classifier used to store the results of the computation
    thread: Option<JoinHandle<()>>, // A thread that is actually doing the computation (so that we can check if it is still running). If none, the computation is done.
    error: Option<String>,          // A string error from the computation
    finished_timestamp: Option<SystemTime>, // A timestamp when the computation was completed (if done)
}

impl Computation {
    pub fn start_timestamp(&self) -> u128 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .expect("Time error")
            .as_millis()
    }

    pub fn end_timestamp(&self) -> Option<u128> {
        self.finished_timestamp.map(|t| {
            t.duration_since(UNIX_EPOCH)
                .expect("Time error")
                .as_millis()
        })
    }
}

lazy_static! {
    static ref COMPUTATION: Arc<RwLock<Option<Computation>>> = Arc::new(RwLock::new(None));
    static ref CHECK_UPDATE_FUNCTION_LOCK: Arc<RwLock<bool>> = Arc::new(RwLock::new(true));
    static ref TREE: Arc<RwLock<Option<Bdt>>> = Arc::new(RwLock::new(None));
}

/// Decision tree API design:
///    - /get_bifurcation_tree: Obtain the full tree currently managed by the server.
///    Initially, this is just the root node, however, it can also be a full tree, because
///    the client can be refreshed and then it loads the correct data again. Returns array of Tree
///    node objects.
///    - /get_attributes/<node_id>: Obtain a list of attributes that can be applied to an unprocessed
///    node. (This can take a while for large models) Returns an array of attribute objects.
///    - /apply_attribute/<node_id>/<attribute_id>: Apply an attribute to an unprocessed node,
///    replacing it with a decision and adding two new child nodes. Returns an array of tree nodes
///    that have changed (i.e. the unprocessed node is now a decision node and it has two children
///    now)
///    - /revert_decision/<node_id>: Turn a decision node back into unprocessed node. This is done
///    recursively, so any children are deleted as well. Returns a
///    { node: UnprocessedNode, removed: array(usize) } - unprocessed node is the new node that
///    replaces the decision node, removed is an array of node ids that are deleted from the tree.
/// Models:
///    - Tree node (three types):
///      - Leaf node: { type: "leaf", id: usize, class: ClassString, cardinality: f64 }
///      - Decision node: { type: "decision", id: usize, attribute_name: String, left: usize, right: usize }
///      - Unprocessed node: { type: "unprocessed", id: usize, classes: ClassList }
///    - Attribute { id: usize, name: String, gain: f64, left: ClassList, right: ClassList }
///    - Class list: array({ class: ClassString, cardinality: f64 })
///

/// Obtain the graph structure of the decision tree as a list of nodes.
#[get("/get_bifurcation_tree")]
fn get_bifurcation_tree() -> BackendResponse {
    let tree = TREE.clone();
    let tree = tree.read().unwrap();
    if let Some(tree) = &*tree {
        BackendResponse::ok(&tree.to_json().to_string())
    } else {
        BackendResponse::err("No tree present. Run computation first.")
    }
}

#[get("/get_attributes/<node_id>")]
fn get_attributes(node_id: String) -> BackendResponse {
    let tree = TREE.clone();
    let tree = tree.read().unwrap();
    if let Some(tree) = &*tree {
        let node = BdtNodeId::try_from_str(&node_id, tree);
        let node = if let Some(node) = node {
            node
        } else {
            return BackendResponse::err(&format!("Invalid node id {}.", node_id));
        };
        BackendResponse::ok(&tree.attribute_gains_json(node).to_string())
    } else {
        BackendResponse::err("No tree present. Run computation first.")
    }
}

#[get("/get_stability_data/<node_id>/<behaviour_str>")]
fn get_stability_data(node_id: String, behaviour_str: String) -> BackendResponse {
    let behaviour = Behaviour::try_from(behaviour_str.as_str());
    let behaviour = match behaviour {
        Ok(behaviour) => Some(behaviour),
        Err(error) => {
            if behaviour_str == "total" {
                None
            } else {
                return BackendResponse::err(error.as_str());
            }
        }
    };
    // First, extract all colors in that tree node.
    let node_params = {
        let tree = TREE.clone();
        let tree = tree.read().unwrap();
        if let Some(tree) = &*tree {
            let node = BdtNodeId::try_from_str(&node_id, tree);
            let node = if let Some(n) = node {
                n
            } else {
                return BackendResponse::err(&format!("Invalid node id {}.", node_id));
            };
            tree.all_node_params(node)
        } else {
            return BackendResponse::err("No bifurcation tree found.");
        }
    };
    // Then find all attractors of the graph
    let cmp = COMPUTATION.read().unwrap();
    if let Some(cmp) = &*cmp {
        let components = if let Some(classifier) = &cmp.classifier {
            if let Some(behaviour) = behaviour {
                classifier.export_components_with_class(behaviour)
            } else {
                classifier
                    .export_components()
                    .into_iter()
                    .map(|(c, _)| c)
                    .collect()
            }
        } else {
            return BackendResponse::err("No attractor data found.");
        };
        if let Some(graph) = &cmp.graph {
            // Now compute which attractors are actually relevant for the node colors
            let components = components
                .into_iter()
                .filter_map(|attractor| {
                    attractor
                        .intersect_colors(&node_params)
                        .take_if(|it| !it.is_empty())
                })
                .collect::<Vec<_>>();

            if components.is_empty() {
                return BackendResponse::err("No attractors with this property.");
            }

            let stability_data = compute_stability(graph, &components);
            let mut response = JsonValue::new_array();
            for variable in graph.as_network().variables() {
                response
                    .push(object! {
                        "variable": graph.as_network().get_variable_name(variable).clone(),
                        "data": stability_data[&variable].to_json(),
                    })
                    .unwrap();
            }
            BackendResponse::ok(&response.to_string())
        } else {
            BackendResponse::err("No attractor data found.")
        }
    } else {
        BackendResponse::err("No attractor data found.")
    }
}

#[post("/apply_attribute/<node_id>/<attribute_id>")]
fn apply_attribute(node_id: String, attribute_id: String) -> BackendResponse {
    let tree = TREE.clone();
    let mut tree = tree.write().unwrap();
    return if let Some(tree) = tree.as_mut() {
        let node = BdtNodeId::try_from_str(&node_id, tree);
        let node = if let Some(node) = node {
            node
        } else {
            return BackendResponse::err(&format!("Invalid node id {}.", node_id));
        };
        let attribute = AttributeId::try_from_str(&attribute_id, tree);
        let attribute = if let Some(val) = attribute {
            val
        } else {
            return BackendResponse::err(&format!("Invalid attribute id {}.", attribute_id));
        };
        if let Ok((left, right)) = tree.make_decision(node, attribute) {
            let changes = array![
                tree.node_to_json(node),
                tree.node_to_json(left),
                tree.node_to_json(right),
            ];
            BackendResponse::ok(&changes.to_string())
        } else {
            BackendResponse::err("Invalid node or attribute id.")
        }
    } else {
        BackendResponse::err("No tree present. Run computation first.")
    };
}

#[post("/revert_decision/<node_id>")]
fn revert_decision(node_id: String) -> BackendResponse {
    let tree = TREE.clone();
    let mut tree = tree.write().unwrap();
    return if let Some(tree) = tree.as_mut() {
        let node = BdtNodeId::try_from_str(&node_id, tree);
        let node = if let Some(node) = node {
            node
        } else {
            return BackendResponse::err(&format!("Invalid node id {}.", node_id));
        };
        let removed = tree.revert_decision(node);
        let removed = removed
            .into_iter()
            .map(|v| v.to_index())
            .collect::<Vec<_>>();
        let response = object! {
                "node": tree.node_to_json(node),
                "removed": JsonValue::from(removed)
        };
        BackendResponse::ok(&response.to_string())
    } else {
        BackendResponse::err("No tree present. Run computation first.")
    };
}

#[post("/auto_expand/<node_id>/<depth>")]
fn auto_expand(node_id: String, depth: String) -> BackendResponse {
    let depth: u32 = {
        let parsed = depth.parse::<u32>();
        if let Ok(depth) = parsed {
            depth
        } else {
            return BackendResponse::err(&format!("Invalid tree depth: {}", depth));
        }
    };
    if depth > 10 {
        return BackendResponse::err("Maximum allowed depth is 10.");
    }
    let tree = TREE.clone();
    let mut tree = tree.write().unwrap();
    if let Some(tree) = tree.as_mut() {
        let node_id: BdtNodeId = if let Some(node_id) = BdtNodeId::try_from_str(&node_id, tree) {
            node_id
        } else {
            return BackendResponse::err(&format!("Invalid node id {}.", node_id));
        };
        let changed = tree.auto_expand(node_id, depth);
        BackendResponse::ok(&tree.to_json_partial(&changed).to_string())
    } else {
        BackendResponse::err("Cannot modify decision tree.")
    }
}

#[post("/apply_tree_precision/<precision>")]
fn apply_tree_precision(precision: String) -> BackendResponse {
    if let Ok(precision) = precision.parse::<u32>() {
        let tree = TREE.clone();
        let mut tree = tree.write().unwrap();
        if let Some(tree) = tree.as_mut() {
            tree.set_precision(precision);
            BackendResponse::ok("\"ok\"")
        } else {
            BackendResponse::err("Cannot modify decision tree.")
        }
    } else {
        BackendResponse::err("Given precision is not a number.")
    }
}

#[get("/get_tree_precision")]
fn get_tree_precision() -> BackendResponse {
    let tree = TREE.clone();
    let tree = tree.read().unwrap();
    if let Some(tree) = tree.as_ref() {
        BackendResponse::ok(&format!("{}", tree.get_precision()))
    } else {
        BackendResponse::err("Cannot modify decision tree.")
    }
}

fn max_parameter_cardinality(function: &FnUpdate) -> usize {
    match function {
        FnUpdate::Const(_) | FnUpdate::Var(_) => 0,
        FnUpdate::Param(_, args) => args.len(),
        FnUpdate::Not(inner) => max_parameter_cardinality(inner),
        FnUpdate::Binary(_, left, right) => max(
            max_parameter_cardinality(left),
            max_parameter_cardinality(right),
        ),
    }
}

/// Accept a partial model containing only the necessary regulations and one update function.
/// Return cardinality of such model (i.e. the number of instantiations of this update function)
/// or error if the update function (or model) is invalid.
#[post("/check_update_function", format = "plain", data = "<data>")]
fn check_update_function(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model size to 10MB
    let mut model_string = String::new();
    return match stream.read_to_string(&mut model_string) {
        Ok(_) => {
            let lock = CHECK_UPDATE_FUNCTION_LOCK.clone();
            let mut lock = lock.write().unwrap();
            let start = SystemTime::now();
            let graph = BooleanNetwork::try_from(model_string.as_str())
                .and_then(|model| {
                    let mut max_size = 0;
                    for v in model.variables() {
                        if let Some(update_function) = model.get_update_function(v) {
                            max_size = max(max_size, max_parameter_cardinality(update_function));
                        } else {
                            max_size = max(max_size, model.regulators(v).len())
                        }
                    }
                    if max_size <= 5 {
                        println!(
                            "Start partial function analysis. {} variables and complexity {}.",
                            model.num_vars(),
                            max_size
                        );
                        SymbolicAsyncGraph::new(model)
                    } else {
                        Err("Function too large for on-the-fly analysis.".to_string())
                    }
                })
                .map(|g| g.unit_colors().approx_cardinality());
            println!(
                "Elapsed: {}, result {:?}",
                start.elapsed().unwrap().as_millis(),
                graph
            );
            (*lock) = !(*lock);
            match graph {
                Ok(cardinality) => {
                    BackendResponse::ok(&format!("{{\"cardinality\":\"{}\"}}", cardinality))
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[get("/ping")]
fn ping() -> BackendResponse {
    println!("...ping...");
    let mut response = object! {
        "timestamp" => json::Null,          // if there is some computation (not necessarily running, this is the time when it started
        "is_cancelled" => false,            // true if the computation has been canceled
        "running" => false,                 // true if the computation thread is still alive
        "progress" => "unknown".to_string(),// arbitrary progress string
        "error" => json::Null,              // arbitrary error string - currently not really used
        "num_classes" => json::Null,        // number of discovered classes so far
        "version" => VERSION.to_string(),   // current compute engine version for compatibility validation
    };
    {
        // Read data from current computation if available...
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(computation) = &*cmp {
            response["timestamp"] = (computation.start_timestamp() as u64).into();
            response["is_cancelled"] = computation.task.is_cancelled().into();
            response["progress"] = computation.task.get_percent_string().into();
            response["is_running"] = computation.thread.is_some().into();
            if let Some(error) = &computation.error {
                response["error"] = error.clone().into();
            }
            if let Some(classes) = computation
                .classifier
                .as_ref()
                .map(|c| c.try_get_num_classes())
            {
                response["num_classes"] = classes.into();
            }
        }
    }
    BackendResponse::ok(&response.to_string())
}

// Try to obtain current class data or none if classifier is busy
/*fn try_get_result(classifier: &Classifier) -> Option<HashMap<Class, BddParams>> {
    for _ in 0..5 {
        if let Some(data) = classifier.try_export_result() {
            return Some(data);
        }
        // wait a little - maybe the lock will become free
        std::thread::sleep(Duration::new(1, 0));
    }
    return None;
}*/

fn try_get_class_params(classifier: &Classifier, class: &Class) -> Option<Option<GraphColors>> {
    for _ in 0..5 {
        if let Some(data) = classifier.try_get_params(class) {
            return Some(data);
        }
        // wait a little - maybe the lock will become free
        std::thread::sleep(Duration::new(1, 0));
    }
    None
}

#[get("/get_results")]
fn get_results() -> BackendResponse {
    let is_partial;
    let (data, elapsed) = {
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            is_partial = cmp.thread.is_some();
            if let Some(classes) = &cmp.classifier {
                let mut result = None;
                for _ in 0..5 {
                    if let Some(data) = classes.try_export_result() {
                        result = Some(data);
                        break;
                    }
                    // wait a little - maybe the lock will become free
                    std::thread::sleep(Duration::new(1, 0));
                }
                if let Some(result) = result {
                    (
                        result,
                        cmp.end_timestamp().map(|t| t - cmp.start_timestamp()),
                    )
                } else {
                    return BackendResponse::err(
                        &"Classification running. Cannot export components right now.".to_string(),
                    );
                }
            } else {
                return BackendResponse::err("Results not available yet.");
            }
        } else {
            return BackendResponse::err("No results available.");
        }
    };
    let lines: Vec<String> = data
        .iter()
        .map(|(c, p)| {
            format!(
                "{{\"sat_count\":{},\"phenotype\":{}}}",
                p.approx_cardinality(),
                c
            )
        })
        .collect();

    println!("Result {:?}", lines);

    let elapsed = if let Some(e) = elapsed { e } else { 0 };

    let mut json = String::new();
    for line in lines.iter().take(lines.len() - 1) {
        json += &format!("{},", line);
    }
    json = format!(
        "{{ \"isPartial\":{}, \"data\":[{}{}], \"elapsed\":{} }}",
        is_partial,
        json,
        lines.last().unwrap(),
        elapsed,
    );

    BackendResponse::ok(&json)
}

#[get("/get_tree_witness/<node_id>")]
fn get_tree_witness(node_id: String) -> BackendResponse {
    let tree = TREE.clone();
    let tree = tree.read().unwrap();
    return if let Some(tree) = &*tree {
        let node = BdtNodeId::try_from_str(&node_id, tree);
        let node = if let Some(node) = node {
            node
        } else {
            return BackendResponse::err(&format!("Invalid node id {}.", node_id));
        };

        if let Some(params) = tree.params_for_leaf(node) {
            get_witness_network(params)
        } else {
            BackendResponse::err("Given node is not an unprocessed node.")
        }
    } else {
        BackendResponse::err("No tree present. Run computation first.")
    };
}

#[get("/get_stability_witness/<node_id>/<behaviour_str>/<variable_str>/<vector_str>")]
fn get_stability_witness(
    node_id: String,
    behaviour_str: String,
    variable_str: String,
    vector_str: String,
) -> BackendResponse {
    let behaviour = Behaviour::try_from(behaviour_str.as_str());
    let behaviour = match behaviour {
        Ok(behaviour) => Some(behaviour),
        Err(error) => {
            if behaviour_str == "total" {
                None
            } else {
                return BackendResponse::err(error.as_str());
            }
        }
    };
    let vector = StabilityVector::try_from(vector_str.as_str());
    let vector = match vector {
        Ok(vector) => vector,
        Err(error) => {
            return BackendResponse::err(error.as_str());
        }
    };
    // First, extract all colors in that tree node.
    let node_params = {
        let tree = TREE.clone();
        let tree = tree.read().unwrap();
        if let Some(tree) = &*tree {
            let node = BdtNodeId::try_from_str(&node_id, tree);
            let node = if let Some(n) = node {
                n
            } else {
                return BackendResponse::err(&format!("Invalid node id {}.", node_id));
            };
            tree.all_node_params(node)
        } else {
            return BackendResponse::err("No bifurcation tree found.");
        }
    };
    // Then find all attractors of the graph
    let cmp = COMPUTATION.read().unwrap();
    if let Some(cmp) = &*cmp {
        let components = if let Some(classifier) = &cmp.classifier {
            if let Some(behaviour) = behaviour {
                classifier.export_components_with_class(behaviour)
            } else {
                classifier
                    .export_components()
                    .into_iter()
                    .map(|(c, _)| c)
                    .collect()
            }
        } else {
            return BackendResponse::err("No attractor data found.");
        };
        if let Some(graph) = &cmp.graph {
            let variable = graph
                .as_network()
                .as_graph()
                .find_variable(variable_str.as_str());
            let variable = if let Some(variable) = variable {
                variable
            } else {
                return BackendResponse::err(
                    format!("Unknown graph variable `{}`.", variable_str).as_str(),
                );
            };

            // Now compute which attractors are actually relevant for the node colors
            let components = components
                .into_iter()
                .filter_map(|attractor| {
                    attractor
                        .intersect_colors(&node_params)
                        .take_if(|it| !it.is_empty())
                })
                .collect::<Vec<_>>();

            let variable_stability =
                VariableStability::for_attractors(graph, &components, variable);
            if let Some(colors) = &variable_stability[vector] {
                get_witness_network(colors)
            } else {
                return BackendResponse::err(
                    format!("No witness available for vector `{}`.", vector_str).as_str(),
                );
            }
        } else {
            BackendResponse::err("No attractor data found.")
        }
    } else {
        BackendResponse::err("No attractor data found.")
    }
}

#[get("/get_witness/<class_str>")]
fn get_witness(class_str: String) -> BackendResponse {
    let mut class = Class::new_empty();
    for char in class_str.chars() {
        match char {
            'D' => class.extend(Behaviour::Disorder),
            'O' => class.extend(Behaviour::Oscillation),
            'S' => class.extend(Behaviour::Stability),
            _ => {
                return BackendResponse::err("Invalid class.");
            }
        }
    }
    {
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if let Some(classifier) = &cmp.classifier {
                if let Some(has_class) = try_get_class_params(classifier, &class) {
                    if let Some(class) = has_class {
                        get_witness_network(&class)
                    } else {
                        BackendResponse::err("Specified class has no witness.")
                    }
                } else {
                    BackendResponse::err(
                        &"Classification in progress. Cannot extract witness right now."
                            .to_string(),
                    )
                }
            } else {
                BackendResponse::err("No results available.")
            }
        } else {
            BackendResponse::err("No results available.")
        }
    }
}

fn get_witness_network(colors: &GraphColors) -> BackendResponse {
    let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
    let cmp = cmp.read().unwrap();
    if let Some(cmp) = &*cmp {
        if let Some(graph) = &cmp.graph {
            let witness = graph.pick_witness(colors);
            let layout = read_layout(cmp.input_model.as_str());
            let mut model_string = format!("{}", witness); // convert back to aeon
            model_string += "\n";
            for (var, (x, y)) in layout {
                model_string += format!("#position:{}:{},{}\n", var, x, y).as_str();
            }
            let (name, description) = read_metadata(cmp.input_model.as_str());
            if let Some(name) = name {
                model_string += format!("#name:{}\n", name).as_str();
            }
            if let Some(description) = description {
                model_string += format!("#description:{}\n", description).as_str();
            }
            BackendResponse::ok(&object! { "model" => model_string }.to_string())
        } else {
            BackendResponse::err("No results available.")
        }
    } else {
        BackendResponse::err("No results available.")
    }
}

#[get("/get_tree_attractors/<node_id>")]
fn get_tree_attractors(node_id: String) -> BackendResponse {
    let tree = TREE.clone();
    let tree = tree.read().unwrap();
    return if let Some(tree) = &*tree {
        let node = BdtNodeId::try_from_str(&node_id, tree);
        let node = if let Some(value) = node {
            value
        } else {
            return BackendResponse::err(&format!("Invalid node id {}.", node_id));
        };

        if let Some(params) = tree.params_for_leaf(node) {
            get_witness_attractors(params)
        } else {
            BackendResponse::err("Given node is not an unprocessed node.")
        }
    } else {
        BackendResponse::err("No tree present. Run computation first.")
    };
}

#[get("/get_stability_attractors/<node_id>/<behaviour_str>/<variable_str>/<vector_str>")]
fn get_stability_attractors(
    node_id: String,
    behaviour_str: String,
    variable_str: String,
    vector_str: String,
) -> BackendResponse {
    let behaviour = Behaviour::try_from(behaviour_str.as_str());
    let behaviour = match behaviour {
        Ok(behaviour) => Some(behaviour),
        Err(error) => {
            if behaviour_str == "total" {
                None
            } else {
                return BackendResponse::err(error.as_str());
            }
        }
    };
    let vector = StabilityVector::try_from(vector_str.as_str());
    let vector = match vector {
        Ok(vector) => vector,
        Err(error) => {
            return BackendResponse::err(error.as_str());
        }
    };
    // First, extract all colors in that tree node.
    let node_params = {
        let tree = TREE.clone();
        let tree = tree.read().unwrap();
        if let Some(tree) = &*tree {
            let node = BdtNodeId::try_from_str(&node_id, tree);
            let node = if let Some(n) = node {
                n
            } else {
                return BackendResponse::err(&format!("Invalid node id {}.", node_id));
            };
            tree.all_node_params(node)
        } else {
            return BackendResponse::err("No bifurcation tree found.");
        }
    };
    // Then find all attractors of the graph
    let cmp = COMPUTATION.read().unwrap();
    if let Some(cmp) = &*cmp {
        let components = if let Some(classifier) = &cmp.classifier {
            if let Some(behaviour) = behaviour {
                classifier.export_components_with_class(behaviour)
            } else {
                classifier
                    .export_components()
                    .into_iter()
                    .map(|(c, _)| c)
                    .collect()
            }
        } else {
            return BackendResponse::err("No attractor data found.");
        };
        if let Some(graph) = &cmp.graph {
            let variable = graph
                .as_network()
                .as_graph()
                .find_variable(variable_str.as_str());
            let variable = if let Some(variable) = variable {
                variable
            } else {
                return BackendResponse::err(
                    format!("Unknown graph variable `{}`.", variable_str).as_str(),
                );
            };

            // Now compute which attractors are actually relevant for the node colors
            let components = components
                .into_iter()
                .filter_map(|attractor| {
                    attractor
                        .intersect_colors(&node_params)
                        .take_if(|it| !it.is_empty())
                })
                .collect::<Vec<_>>();

            let variable_stability =
                VariableStability::for_attractors(graph, &components, variable);
            if let Some(colors) = &variable_stability[vector] {
                get_witness_attractors(colors)
            } else {
                return BackendResponse::err(
                    format!("No witness available for vector `{}`.", vector_str).as_str(),
                );
            }
        } else {
            BackendResponse::err("No attractor data found.")
        }
    } else {
        BackendResponse::err("No attractor data found.")
    }
}

type EdgeList = Vec<(ArrayBitVector, ArrayBitVector)>;

#[get("/get_attractors/<class_str>")]
fn get_attractors(class_str: String) -> BackendResponse {
    let mut class = Class::new_empty();
    for char in class_str.chars() {
        match char {
            'D' => class.extend(Behaviour::Disorder),
            'O' => class.extend(Behaviour::Oscillation),
            'S' => class.extend(Behaviour::Stability),
            _ => {
                return BackendResponse::err("Invalid class.");
            }
        }
    }
    {
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if let Some(classifier) = &cmp.classifier {
                if let Some(has_class) = try_get_class_params(classifier, &class) {
                    if let Some(class) = has_class {
                        get_witness_attractors(&class)
                    } else {
                        BackendResponse::err("Specified class has no witness.")
                    }
                } else {
                    BackendResponse::err(
                        &"Classification still in progress. Cannot explore attractors now."
                            .to_string(),
                    )
                }
            } else {
                BackendResponse::err("No results available.")
            }
        } else {
            BackendResponse::err("No results available.")
        }
    }
}

fn get_witness_attractors(f_colors: &GraphColors) -> BackendResponse {
    {
        // Variables prefixed with f_ are from the original fully parametrised graph.
        let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if let Some(f_classifier) = &cmp.classifier {
                if let Some(graph) = &cmp.graph {
                    let f_witness_colour = f_colors.pick_singleton();
                    let witness_network: BooleanNetwork = graph.pick_witness(&f_witness_colour);
                    let witness_graph = SymbolicAsyncGraph::new(witness_network.clone()).unwrap();
                    let witness_str = witness_network.to_string();
                    let f_witness_attractors = f_classifier.attractors(&f_witness_colour);
                    let variable_name_strings = witness_network
                        .variables()
                        .map(|id| format!("\"{}\"", witness_network.get_variable_name(id)));

                    let mut all_attractors: Vec<(Behaviour, EdgeList, HashSet<usize>)> = Vec::new();

                    // Note that the choice of graph/witness_graph is not arbitrary.
                    // The attractor set is from the original graph, but source_set/target_set
                    // are based on the witness_graph. This means they have different number
                    // of BDD variables inside!
                    let mut has_large_attractors = false;
                    for (f_attractor, behaviour) in f_witness_attractors.iter() {
                        println!(
                            "Attractor {:?} state count: {}",
                            behaviour,
                            f_attractor.approx_cardinality()
                        );
                        let mut attractor_graph: Vec<(ArrayBitVector, ArrayBitVector)> = Vec::new();
                        let mut not_fixed_vars: HashSet<usize> = HashSet::new();
                        if *behaviour == Behaviour::Stability {
                            // This is a sink - no edges
                            assert_eq!(f_attractor.materialize().iter().count(), 1);
                            let sink: ArrayBitVector =
                                f_attractor.materialize().iter().next().unwrap();
                            attractor_graph.push((sink.clone(), sink));
                            for i in 0..witness_network.num_vars() {
                                // In sink, we mark everything as "not-fixed" because we want to just display it normally.
                                not_fixed_vars.insert(i);
                            }
                        } else if f_attractor.approx_cardinality() >= 500.0 {
                            has_large_attractors = true;
                            // For large attractors, only show fixed values.
                            let mut state_0 =
                                ArrayBitVector::from(vec![false; graph.as_network().num_vars()]);
                            let mut state_1 =
                                ArrayBitVector::from(vec![true; graph.as_network().num_vars()]);
                            for var in graph.as_network().variables() {
                                let f_var_true = graph.fix_network_variable(var, true).vertices();
                                let f_var_false = graph.fix_network_variable(var, false).vertices();
                                let f_always_one = f_attractor.intersect(&f_var_false).is_empty();
                                let f_always_zero = f_attractor.intersect(&f_var_true).is_empty();
                                if f_always_one {
                                    state_0.set(var.into(), true);
                                } else if f_always_zero {
                                    state_1.set(var.into(), false);
                                } else {
                                    not_fixed_vars.insert(var.into());
                                }
                            }
                            attractor_graph.push((state_0.clone(), state_1.clone()));
                            attractor_graph.push((state_1, state_0));
                        } else {
                            for source in f_attractor.materialize().iter() {
                                let source_set = witness_graph.vertex(&source);
                                let mut target_set = witness_graph.mk_empty_vertices();
                                for v in witness_graph.as_network().variables() {
                                    let post = witness_graph.var_post(v, &source_set);
                                    if !post.is_empty() {
                                        not_fixed_vars.insert(v.into());
                                        target_set = target_set.union(&post);
                                    }
                                }

                                for target in target_set.vertices().materialize().iter() {
                                    attractor_graph.push((source.clone(), target));
                                }
                            }
                        }

                        all_attractors.push((*behaviour, attractor_graph, not_fixed_vars));
                    }

                    // now the data is stored in `all_attractors`, just convert it to json:
                    let mut json = String::new();

                    for (i, (behavior, graph, not_fixed)) in all_attractors.iter().enumerate() {
                        if i != 0 {
                            json += ",";
                        } // json? no trailing commas for you
                        json += &format!("{{\"class\":\"{:?}\", \"graph\":[", behavior);
                        let mut edge_count = 0;
                        for (j, edge) in graph.iter().enumerate() {
                            fn state_to_binary(
                                state: &ArrayBitVector,
                                not_fixed: &HashSet<usize>,
                            ) -> String {
                                let mut result = String::new();
                                for i in 0..state.len() {
                                    if not_fixed.contains(&i) {
                                        result.push(if state.get(i) { '1' } else { '0' });
                                    } else {
                                        result.push(if state.get(i) { '⊤' } else { '⊥' });
                                    }
                                }
                                result
                            }
                            let from: String = state_to_binary(&edge.0, not_fixed);
                            let to: String = state_to_binary(&edge.1, not_fixed);
                            if j != 0 {
                                json += ","
                            }
                            json += &format!("[\"{}\", \"{}\"]", from, to);
                            edge_count += 1;
                        }
                        json += &format!("], \"edges\":{}}}", edge_count);
                    }
                    json = "{ \"attractors\":[".to_owned() + &json + "], \"variables\":[";
                    for (i, var) in variable_name_strings.enumerate() {
                        if i != 0 {
                            json += ",";
                        }
                        json += var.as_str();
                    }
                    json += &format!(
                        "], \"model\":{}, \"has_large_attractors\": {}",
                        &object! { "model" => witness_str }.to_string(),
                        has_large_attractors
                    );
                    BackendResponse::ok(&(json + "}"))
                } else {
                    BackendResponse::err("No results available.")
                }
            } else {
                BackendResponse::err("No results available.")
            }
        } else {
            BackendResponse::err("No results available.")
        }
    }
}

/// Accept an Aeon model, parse it and start a new computation (if there is no computation running).
#[post("/start_computation", format = "plain", data = "<data>")]
fn start_computation(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => {
            // First, try to parse the network so that the user can at least verify it is correct...
            match BooleanNetwork::try_from(aeon_string.as_str()) {
                Ok(network) => {
                    // Now we can try to start the computation...
                    let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
                    {
                        // First, just try to read the computation, if there is something
                        // there, we just want to quit fast...
                        let cmp = cmp.read().unwrap();
                        if let Some(computation) = &*cmp {
                            if computation.thread.is_some() {
                                return BackendResponse::err("Previous computation is still running. Cancel it before starting a new one.");
                            }
                        }
                    }
                    {
                        // Now actually get the write lock, but check again because race conditions...
                        let mut cmp = cmp.write().unwrap();
                        if let Some(computation) = &*cmp {
                            if computation.thread.is_some() {
                                return BackendResponse::err("Previous computation is still running. Cancel it before starting a new one.");
                            }
                        }
                        let mut new_cmp = Computation {
                            timestamp: SystemTime::now(),
                            task: GraphTaskContext::new(),
                            input_model: aeon_string.clone(),
                            graph: None,
                            classifier: None,
                            thread: None,
                            error: None,
                            finished_timestamp: None,
                        };
                        // Prepare thread - not that we have computation locked, so the thread
                        // will have to wait for us to end before writing down the graph and other
                        // stuff.
                        let cmp_thread = std::thread::spawn(move || {
                            let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
                            match SymbolicAsyncGraph::new(network) {
                                Ok(graph) => {
                                    // Now that we have graph, we can create classifier and progress
                                    // and save them into the computation.
                                    let classifier = Arc::new(Classifier::new(&graph));
                                    let graph = Arc::new(graph);
                                    {
                                        if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                            cmp.graph = Some(graph.clone());
                                            cmp.classifier = Some(classifier.clone());
                                        } else {
                                            panic!("Cannot save graph. No computation found.")
                                        }
                                    }

                                    if let Some(cmp) = cmp.read().unwrap().as_ref() {
                                        // TODO: Note that this holds the read-lock on computation
                                        // for the  whole time, which is mostly ok because it can be
                                        // cancelled without write-lock, but we should find a
                                        // way to avoid this!
                                        let task_context = &cmp.task;
                                        task_context.restart(&graph);

                                        // Now we can actually start the computation...

                                        // First, perform ITGR reduction.
                                        let (universe, active_variables) =
                                            interleaved_transition_guided_reduction(
                                                task_context,
                                                &graph,
                                                graph.mk_unit_colored_vertices(),
                                            );

                                        // Then run Xie-Beerel to actually detect the components.
                                        xie_beerel_attractors(
                                            task_context,
                                            &graph,
                                            &universe,
                                            &active_variables,
                                            |component| {
                                                println!(
                                                    "Component {}",
                                                    component.approx_cardinality()
                                                );
                                                classifier.add_component(component, &graph);
                                            },
                                        );
                                    }

                                    {
                                        if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                            cmp.finished_timestamp = Some(SystemTime::now());
                                        } else {
                                            panic!(
                                                "Cannot finish computation. No computation found."
                                            )
                                        }
                                    }

                                    {
                                        let result = classifier.export_result();
                                        let tree = TREE.clone();
                                        let mut tree = tree.write().unwrap();
                                        *tree = Some(Bdt::new_from_graph(result, &graph));
                                        println!("Saved decision tree");
                                    }

                                    println!("Component search done...");
                                }
                                Err(error) => {
                                    if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                        cmp.error = Some(error);
                                    } else {
                                        panic!(
                                            "Cannot save computation error. No computation found."
                                        )
                                    }
                                }
                            }
                            {
                                // Remove reference to thread, since we are done now...
                                if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                    cmp.thread = None;
                                } else {
                                    panic!("Cannot finalize thread. No computation found.");
                                };
                            }
                        });
                        new_cmp.thread = Some(cmp_thread);

                        let start = new_cmp.start_timestamp();
                        // Now write the new computation to the global state...
                        *cmp = Some(new_cmp);

                        BackendResponse::ok(&object! { "timestamp" => start as u64 }.to_string())
                        // status of the computation can be obtained via ping...
                    }
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

#[post("/cancel_computation", format = "plain")]
fn cancel_computation() -> BackendResponse {
    let cmp: Arc<RwLock<Option<Computation>>> = COMPUTATION.clone();
    {
        // first just check there is something to cancel
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if cmp.thread.is_none() {
                return BackendResponse::err("Nothing to cancel. Computation already done.");
            }
            if cmp.task.is_cancelled() {
                return BackendResponse::err("Computation already cancelled.");
            }
        } else {
            return BackendResponse::err("No computation to cancel.");
        }
    }
    let cmp = cmp.read().unwrap();
    if let Some(cmp) = &*cmp {
        if cmp.thread.is_none() {
            return BackendResponse::err("Nothing to cancel. Computation already done.");
        }
        if cmp.task.cancel() {
            BackendResponse::ok(&"\"ok\"".to_string())
        } else {
            BackendResponse::err("Computation already cancelled.")
        }
    } else {
        BackendResponse::err("No computation to cancel.")
    }
}

/// Accept an SBML (XML) file and try to parse it into a `BooleanNetwork`.
/// If everything goes well, return a standard result object with a parsed model, or
/// error if something fails.
#[post("/sbml_to_aeon", format = "plain", data = "<data>")]
fn sbml_to_aeon(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut sbml_string = String::new();
    match stream.read_to_string(&mut sbml_string) {
        Ok(_) => {
            match BooleanNetwork::try_from_sbml(&sbml_string) {
                Ok((model, layout)) => {
                    let mut model_string = format!("{}", model); // convert back to aeon
                    model_string += "\n";
                    for (var, (x, y)) in layout {
                        model_string += format!("#position:{}:{},{}\n", var, x, y).as_str();
                    }
                    BackendResponse::ok(&object! { "model" => model_string }.to_string())
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    }
}

/// Try to read the model layout metadata from the given aeon file.
fn read_layout(aeon_string: &str) -> HashMap<String, (f64, f64)> {
    let re = Regex::new(r"^\s*#position:(?P<var>[a-zA-Z0-9_]+):(?P<x>.+?),(?P<y>.+?)\s*$").unwrap();
    let mut layout = HashMap::new();
    for line in aeon_string.lines() {
        if let Some(captures) = re.captures(line) {
            let var = captures["var"].to_string();
            let x = captures["x"].parse::<f64>();
            let y = captures["y"].parse::<f64>();
            if let (Ok(x), Ok(y)) = (x, y) {
                layout.insert(var, (x, y));
            }
        }
    }
    layout
}

fn read_metadata(aeon_string: &str) -> (Option<String>, Option<String>) {
    let mut model_name = None;
    let mut model_description = None;
    let name_regex = Regex::new(r"^\s*#name:(?P<name>.+)$").unwrap();
    let description_regex = Regex::new(r"^\s*#description:(?P<desc>.+)$").unwrap();
    for line in aeon_string.lines() {
        if let Some(captures) = name_regex.captures(line) {
            model_name = Some(captures["name"].to_string());
        }
        if let Some(captures) = description_regex.captures(line) {
            model_description = Some(captures["desc"].to_string());
        }
    }
    (model_name, model_description)
}

/// Accept an Aeon file, try to parse it into a `BooleanNetwork`
/// which will then be translated into SBML (XML) representation.
/// Preserve layout metadata.
#[post("/aeon_to_sbml", format = "plain", data = "<data>")]
fn aeon_to_sbml(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut aeon_string = String::new();
    match stream.read_to_string(&mut aeon_string) {
        Ok(_) => match BooleanNetwork::try_from(aeon_string.as_str()) {
            Ok(network) => {
                let layout = read_layout(&aeon_string);
                let sbml_string = network.to_sbml(Some(&layout));
                BackendResponse::ok(&object! { "model" => sbml_string }.to_string())
            }
            Err(error) => BackendResponse::err(&error),
        },
        Err(error) => BackendResponse::err(&format!("{}", error)),
    }
}

/// Accept an Aeon file and create an SBML version with all parameters instantiated (a witness model).
/// Note that this can take quite a while for large models since we have to actually build
/// the unit BDD right now (in the future, we might opt to use a SAT solver which might be faster).
#[post("/aeon_to_sbml_instantiated", format = "plain", data = "<data>")]
fn aeon_to_sbml_instantiated(data: Data) -> BackendResponse {
    let mut stream = data.open().take(10_000_000); // limit model to 10MB
    let mut aeon_string = String::new();
    return match stream.read_to_string(&mut aeon_string) {
        Ok(_) => {
            match BooleanNetwork::try_from(aeon_string.as_str()).and_then(SymbolicAsyncGraph::new) {
                Ok(graph) => {
                    let witness = graph.pick_witness(graph.unit_colors());
                    let layout = read_layout(&aeon_string);
                    BackendResponse::ok(
                        &object! { "model" => witness.to_sbml(Some(&layout)) }.to_string(),
                    )
                }
                Err(error) => BackendResponse::err(&error),
            }
        }
        Err(error) => BackendResponse::err(&format!("{}", error)),
    };
}

fn main() {
    //test_main::run();
    let address = std::env::var("AEON_ADDR").unwrap_or_else(|_| "localhost".to_string());
    let port_from_args = {
        let mut args = std::env::args();
        args.next(); // Skip binary path
        args.next().and_then(|s| s.parse::<u16>().ok())
    };
    let port_from_env = std::env::var("AEON_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok());
    let port: u16 = port_from_args.or(port_from_env).unwrap_or(8000);
    let config = Config::build(Environment::Production)
        .address(address)
        .port(port)
        .finalize();

    rocket::custom(config.unwrap())
        .mount(
            "/",
            routes![
                ping,
                start_computation,
                cancel_computation,
                get_results,
                get_witness,
                get_tree_witness,
                get_attractors,
                get_tree_attractors,
                get_stability_data,
                get_stability_attractors,
                get_stability_witness,
                check_update_function,
                sbml_to_aeon,
                aeon_to_sbml,
                aeon_to_sbml_instantiated,
                get_bifurcation_tree,
                get_attributes,
                apply_attribute,
                revert_decision,
                apply_tree_precision,
                get_tree_precision,
                auto_expand,
            ],
        )
        .launch();
}

struct BackendResponse {
    message: String,
}

impl BackendResponse {
    fn ok(message: &str) -> Self {
        BackendResponse {
            message: format!("{{ \"status\": true, \"result\": {} }}", message),
        }
    }

    fn err(message: &str) -> Self {
        BackendResponse {
            message: object! {
            "status" => false,
            "message" => message.replace("\n", "<br>"),
            }
            .to_string(),
        }
    }
}

impl<'r> Responder<'r> for BackendResponse {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        use std::io::Cursor;

        let cursor = Cursor::new(self.message);
        Response::build()
            .header(ContentType::Plain)
            .header(AccessControlAllowOrigin::Any)
            // This magic set of headers might fix some CROS issues, but we are not sure yet...
            .header(Header::new("Allow", "GET, POST, OPTIONS, PUT, DELETE"))
            .header(Header::new("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, DELETE"))
            .header(Header::new("Access-Control-Allow-Headers", "X-API-KEY, Origin, X-Requested-With, Content-Type, Accept, Access-Control-Request-Method"))
            .sized_body(cursor)
            .ok()
    }
}
