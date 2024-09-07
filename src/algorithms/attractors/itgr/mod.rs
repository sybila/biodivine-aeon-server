use crate::algorithms::attractors::itgr::itgr_process::ItgrProcess;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::time::{Duration, SystemTime};

mod itgr_process;
mod reachability_process;

pub async fn schedule_reductions(
    stg: SymbolicAsyncGraph,
    fork_limit: usize,
) -> GraphColoredVertices {
    let mut processes = stg
        .variables()
        .map(|var| ItgrProcess::new(&stg, var))
        .collect::<Vec<_>>();

    let mut universe = (stg.mk_unit_colored_vertices(), 0);

    let mut futures = FuturesUnordered::new();

    while !processes.is_empty() || !futures.is_empty() {
        // Put processes with the smallest weight at the end of the vector.
        processes.sort_by_cached_key(|p| -(p.weight() as isize));

        while futures.len() < fork_limit && !processes.is_empty() {
            let mut process = processes.pop().unwrap();

            // Before queueing up the process, check if it has all the latest updates.
            if process.last_timestamp() < universe.1 {
                process.restrict(&universe.0, universe.1);
            }

            futures.push(tokio::spawn(async move {
                let start = SystemTime::now();
                let mut iter = 0;
                loop {
                    iter += 1;
                    let (done, to_remove) = process.step().await;
                    if done
                        || to_remove.is_some()
                        || start.elapsed().unwrap() > Duration::from_secs(30)
                    {
                        println!("Task completed {} iterations.", iter);
                        return (done, to_remove, process);
                    }
                }
            }));
        }

        let result = futures.next().await.unwrap();
        let (done, to_remove, process) = result.unwrap();

        if let Some(to_remove) = to_remove {
            universe.0 = universe.0.minus(&to_remove);
            universe.1 += 1;
        }

        if !done {
            processes.push(process);
        }

        println!("Remaining: {} + {}", processes.len(), futures.len());
    }

    universe.0
    /*let mut futures = Vec::new();

    while !processes.is_empty() || !futures.is_empty() {
        // Put processes with the smallest weight at the end of the vector.
        processes.sort_by_cached_key(|p| -(p.weight() as isize));

        while futures.len() < fork_limit && !processes.is_empty() {
            let mut process = processes.pop().unwrap();

            // Before queueing up the process, check if it has all the latest updates.
            if process.last_timestamp() < universe.1 {
                process.restrict(&universe.0, universe.1);
            }

            futures.push(tokio::spawn(async move {
                let (done, to_remove) = process.step().await;
                (done, to_remove, process)
            }));
        }

        let (result, _, remaining) = futures::future::select_all(futures.into_iter()).await;
        futures = remaining;

        let (done, to_remove, process) = result.unwrap();

        if let Some(to_remove) = to_remove {
            universe.0 = universe.0.minus(&to_remove);
            universe.1 = universe.1 + 1;
        }

        if !done {
            processes.push(process);
        }

        println!("Remaining: {}", processes.len());
    }

    universe.0*/
}
