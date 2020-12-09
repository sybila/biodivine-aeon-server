use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::{BooleanNetwork, FnUpdate};
use std::collections::HashSet;
use std::convert::TryFrom;

fn main() {
    //let args: Vec<String> = std::env::args().collect();
    //let auto_fix = args.get(1).map(|s| s == "auto_fix").unwrap_or(false);
    let auto_fix = true;
    let benchmarks = std::fs::read_dir("./benchmark").unwrap();

    let mut errors = 0;
    for bench_dir in benchmarks {
        let bench_dir = bench_dir.unwrap();
        if !bench_dir.file_type().unwrap().is_dir() {
            eprintln!("SKIP: {} is not a directory.", bench_dir.path().display());
            continue;
        }
        let readme_path = bench_dir.path().join("model.txt");
        if !readme_path.exists() {
            errors += 1;
            eprintln!("ERROR: Missing README in {}.", bench_dir.path().display());
        }
        let sbml_model_path = bench_dir.path().join("model.sbml");
        let sbml_model = if !sbml_model_path.exists() {
            errors += 1;
            eprintln!(
                "ERROR: Missing model.sbml in {}.",
                bench_dir.path().display()
            );
            continue;
        } else {
            // Check that the sbml model is readable:
            let model_string = std::fs::read_to_string(sbml_model_path).unwrap();
            let model = BooleanNetwork::from_sbml(&model_string);
            match model {
                Err(err) => {
                    errors += 1;
                    eprintln!(
                        "ERROR: Invalid SBML model in {}.",
                        bench_dir.path().display()
                    );
                    eprintln!("\t\t{}", err);
                    continue;
                }
                Ok((model, _)) => model,
            }
        };
        let aeon_model_path = bench_dir.path().join("model.aeon");
        if !aeon_model_path.exists() {
            errors += 1;
            eprintln!(
                "ERROR: Missing model.aeon in {}.",
                bench_dir.path().display()
            );
        } else {
            // Check that the aeon model is valid:
            let model_string = std::fs::read_to_string(aeon_model_path.clone()).unwrap();
            let model = BooleanNetwork::try_from(model_string.as_str());
            match model {
                Ok(mut model) => {
                    // Check that basic properties match SBML model. But note that variables can be re-ordered...
                    let mut models_match =
                        model.graph().num_vars() == sbml_model.graph().num_vars();
                    if model.graph().num_vars() != sbml_model.graph().num_vars() {
                        eprintln!(
                            "{} != {}",
                            model.graph().num_vars(),
                            sbml_model.graph().num_vars()
                        );
                    }
                    for v in model.graph().variable_ids() {
                        let regulators_in_model: HashSet<String> = model
                            .graph()
                            .regulators(v)
                            .into_iter()
                            .map(|r| model.graph().get_variable(r).get_name().clone())
                            .collect();
                        let regulators_in_sbml_model: HashSet<String> = sbml_model
                            .graph()
                            .regulators(
                                sbml_model
                                    .graph()
                                    .find_variable(model.graph().get_variable(v).get_name())
                                    .unwrap(),
                            )
                            .into_iter()
                            .map(|r| sbml_model.graph().get_variable(r).get_name().clone())
                            .collect();
                        if regulators_in_model != regulators_in_sbml_model {
                            eprintln!(
                                "{:?} != {:?}",
                                regulators_in_model, regulators_in_sbml_model
                            );
                        }
                        models_match =
                            models_match && regulators_in_model == regulators_in_sbml_model;
                    }
                    if !models_match {
                        errors += 1;
                        eprintln!(
                            "ERROR: SBML and AEON model are different in {}.",
                            bench_dir.path().display()
                        );
                    }
                    // Check that all update functions are set (for non-parametrized model anyway).
                    let mut model_ok = true;
                    for v in model.graph().variable_ids() {
                        let function = model.get_update_function(v);
                        if function.is_none() {
                            model_ok = false;
                            model.add_update_function(v, FnUpdate::Const(true)).unwrap();
                        }
                    }
                    if !model_ok {
                        errors += 1;
                        eprintln!(
                            "ERROR: Model in {} contains unconstrained variables.",
                            bench_dir.path().display()
                        );
                        if auto_fix {
                            std::fs::write(aeon_model_path, model.to_string()).unwrap();
                        } else {
                            eprintln!("Fixed model: ");
                            eprintln!("{}", model.to_string());
                        }
                    } else {
                        let graph = SymbolicAsyncGraph::new(model);
                        match graph {
                            Ok(graph) => {
                                if graph.unit_colors().cardinality() != 1.0 {
                                    errors += 1;
                                    eprintln!(
                                        "ERROR: Default model has {} colors in {}.",
                                        graph.unit_colors().cardinality(),
                                        bench_dir.path().display()
                                    );
                                }
                            }
                            Err(err) => {
                                errors += 1;
                                eprintln!(
                                    "ERROR: Cannot build graph from model in {}.",
                                    bench_dir.path().display()
                                );
                                eprintln!("{}", err);
                            }
                        }
                    }
                }
                Err(err) => {
                    errors += 1;
                    eprintln!(
                        "ERROR: Invalid AEON model in {}.",
                        bench_dir.path().display()
                    );
                    eprintln!("\t\t{}", err);
                }
            }
        }
        println!("OK: {}", bench_dir.path().display());
    }
    println!("TOTAL ERRORS: {}", errors);
}
