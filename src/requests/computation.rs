use crate::scc::algo_components::components;
use crate::scc::{Classifier, ProgressTracker};
use crate::{ArcComputation, BackendResponse, Computation};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use json::JsonValue;
use std::convert::TryFrom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Accept an Aeon model, parse it and start a new computation (if there is no computation running).
pub fn start_computation(session: ArcComputation, aeon_string: &str) -> BackendResponse {
    // First, try to parse the network so that the user can at least verify it is correct...
    match BooleanNetwork::try_from(aeon_string) {
        Ok(network) => {
            // Now we can try to start the computation...
            let cmp: Arc<RwLock<Option<Computation>>> = session.clone();
            {
                // First, just try to read the computation, if there is something
                // there, we just want to quit fast...
                let cmp = cmp.read().unwrap();
                if let Some(computation) = &*cmp {
                    if computation.thread.is_some() {
                        return BackendResponse::err(&"Previous computation is still running. Cancel it before starting a new one.".to_string());
                    }
                }
            }
            {
                // Now actually get the write lock, but check again because race conditions...
                let mut cmp = cmp.write().unwrap();
                if let Some(computation) = &*cmp {
                    if computation.thread.is_some() {
                        return BackendResponse::err(&"Previous computation is still running. Cancel it before starting a new one.".to_string());
                    }
                }
                let mut new_cmp = Computation {
                    timestamp: SystemTime::now(),
                    is_cancelled: Arc::new(AtomicBool::new(false)),
                    input_model: aeon_string.to_string(),
                    graph: None,
                    classifier: None,
                    progress: None,
                    thread: None,
                    error: None,
                    finished_timestamp: None,
                };
                let cancelled = new_cmp.is_cancelled.clone();
                // Prepare thread - not that we have computation locked, so the thread
                // will have to wait for us to end before writing down the graph and other
                // stuff.
                let cmp_thread = std::thread::spawn(move || {
                    let cmp: Arc<RwLock<Option<Computation>>> = session.clone();
                    match AsyncGraph::new(network) {
                        Ok(graph) => {
                            // Now that we have graph, we can create classifier and progress
                            // and save them into the computation.
                            let classifier = Arc::new(Classifier::new(&graph));
                            let progress = Arc::new(ProgressTracker::new(&graph));
                            let graph = Arc::new(graph);
                            {
                                if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                    cmp.graph = Some(graph.clone());
                                    cmp.progress = Some(progress.clone());
                                    cmp.classifier = Some(classifier.clone());
                                } else {
                                    panic!("Cannot save graph. No computation found.")
                                }
                            }

                            // Now we can actually start the computation...
                            components(&graph, &progress, &*cancelled, |component| {
                                let size = component.iter().count();
                                println!("Component {}", size);
                                classifier.add_component(component, &graph);
                            });

                            {
                                if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                    cmp.finished_timestamp = Some(SystemTime::now());
                                } else {
                                    panic!("Cannot finish computation. No computation found.")
                                }
                            }

                            println!("Component search done...");
                        }
                        Err(error) => {
                            if let Some(cmp) = cmp.write().unwrap().as_mut() {
                                cmp.error = Some(error);
                            } else {
                                panic!("Cannot save computation error. No computation found.")
                            }
                        }
                    }
                    {
                        // Remove reference to thread, since we are done now...
                        if let Some(cmp) = cmp.write().unwrap().as_mut() {
                            cmp.thread = None;
                        } else {
                            panic!("Cannot finalize thread. No computation found.");
                        }
                    }
                    return ();
                });
                new_cmp.thread = Some(cmp_thread);

                let start = new_cmp.start_timestamp();
                // Now write the new computation to the global state...
                *cmp = Some(new_cmp);

                BackendResponse::ok_json(object! { "timestamp" => start as u64 })
                // status of the computation can be obtained via ping...
            }
        }
        Err(error) => BackendResponse::err(&error),
    }
}

pub fn cancel_computation(cmp: ArcComputation) -> BackendResponse {
    {
        // first just check there is something to cancel
        let cmp = cmp.read().unwrap();
        if let Some(cmp) = &*cmp {
            if cmp.thread.is_none() {
                return BackendResponse::err(
                    &"Nothing to cancel. Computation already done.".to_string(),
                );
            }
            if cmp.is_cancelled.load(Ordering::SeqCst) {
                return BackendResponse::err(&"Computation already cancelled.".to_string());
            }
        } else {
            return BackendResponse::err(&"No computation to cancel.".to_string());
        }
    }
    let cmp = cmp.write().unwrap();
    return if let Some(cmp) = &*cmp {
        if cmp.thread.is_none() {
            return BackendResponse::err(
                &"Nothing to cancel. Computation already done.".to_string(),
            );
        }
        if cmp.is_cancelled.swap(true, Ordering::SeqCst) == false {
            BackendResponse::ok_json(JsonValue::from("ok"))
        } else {
            BackendResponse::err(&"Computation already cancelled.".to_string())
        }
    } else {
        BackendResponse::err(&"No computation to cancel.".to_string())
    };
}
