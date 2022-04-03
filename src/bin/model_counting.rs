use std::collections::HashSet;
use std::io::Read;
use biodivine_lib_param_bn::{BinaryOp, BooleanNetwork, FnUpdate, RegulatoryGraph};
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_aeon_server::GraphTaskContext;
use biodivine_aeon_server::scc::algo_interleaved_transition_guided_reduction::interleaved_transition_guided_reduction;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let mut lines = input.lines()
        .filter(|line| {
            let trimmed = line.trim();
            // Skip comments and empty lines
            !trimmed.is_empty() && !trimmed.starts_with("c")
        }).collect::<Vec<_>>();

    // At least two lines are expected: header and clauses.
    assert!(lines.len() >= 2);
    // Move header to the back.
    lines.reverse();

    let header = lines.pop().unwrap();
    let header = header
        .split(" ")
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();

    assert_eq!(header.len(), 4);

    let var_count = header[2].parse::<usize>().unwrap();

    let var_names = (0..var_count).into_iter()
        .map(|id| format!("x_{}", id + 1))
        .collect::<Vec<_>>();

    let mut rg = RegulatoryGraph::new(var_names);
    let var_ids = rg.variables().collect::<Vec<_>>();

    let clauses = lines.into_iter()
        .map(|line| {
            line.split(" ")
                .filter(|segment| !segment.trim().is_empty())
                .filter(|segment| segment.trim() != "0")
                .map(|segment| segment.parse::<isize>().unwrap())
                .map(|literal| {
                    if literal > 0 {
                        ((literal - 1) as usize, true)
                    } else {
                        (((-1 * literal) - 1) as usize, false)
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut dep_clauses = rg.variables().map(|_| HashSet::new()).collect::<Vec<_>>();

    // Put clause index into dependent clauses per variable.
    for (i, clause) in clauses.iter().enumerate() {
        for (literal, _) in clause {
            let bag = &mut dep_clauses[*literal];
            bag.insert(i);
        }
    }

    // Go through all clause dependencies and create regulations.
    for (target_index, bag) in dep_clauses.iter().enumerate() {
        for clause_index in bag {
            for (regulator_index, _) in &clauses[*clause_index] {
                let reg = rg.find_regulation(var_ids[*regulator_index], var_ids[target_index]);
                if reg.is_none() {
                    rg.add_regulation(
                        format!("x_{}", regulator_index + 1).as_str(),
                        format!("x_{}", target_index + 1).as_str(),
                        false,
                        None
                    ).unwrap();
                }
            }
        }
    }

    let mut bn = BooleanNetwork::new(rg);

    for (target_index, bag) in dep_clauses.iter().enumerate() {
        println!("Variable {:?} depends on {} clause(s).", var_ids[target_index], bag.len());
        if !bag.is_empty() {
            let mut update = FnUpdate::mk_true();
            for clause_index in bag {
                let clause = clauses[*clause_index].iter()
                    .map(|(index, value)| {
                        if *value {
                            FnUpdate::mk_var(var_ids[*index])
                        } else {
                            FnUpdate::mk_not(FnUpdate::mk_var(var_ids[*index]))
                        }
                    })
                    .fold(FnUpdate::mk_false(), |a, b| {
                        FnUpdate::mk_binary(BinaryOp::Or, a, b)
                    });

                update = FnUpdate::mk_binary(BinaryOp::And, update, clause);
            }

            let update = FnUpdate::mk_binary(
                BinaryOp::Iff,
                update,
                FnUpdate::mk_var(var_ids[target_index])
            );

            bn.add_update_function(var_ids[target_index], update).unwrap();
        }
        // Variables with no clauses are left with free updates, but these should be rare.
    }

    println!("Translation done.");

    let stg = SymbolicAsyncGraph::new(bn).unwrap();

    let ctx = GraphTaskContext::new();
    ctx.restart(&stg);

    println!("Whole universe: {}", stg.unit_colored_vertices().approx_cardinality());

    let (universe, vars) = interleaved_transition_guided_reduction(
        &ctx,
        &stg,
        stg.mk_unit_colored_vertices(),
    );

    println!("Reduced universe: {} (vars {})", universe.approx_cardinality(), vars.len());
}