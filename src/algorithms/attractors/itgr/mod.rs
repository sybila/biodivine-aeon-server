use std::cmp::min;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use tokio::sync::{Notify, OwnedSemaphorePermit, RwLock, Semaphore, SemaphorePermit};
use crate::algorithms::attractors::itgr::itgr_process::ItgrProcess;

mod reachability_process;
mod itgr_process;

struct Scheduler {
    fork_limit: usize,
    // Processes that exceed this weight are automatically parked.
    weight_limit: AtomicUsize,
    // Barrier where all tasks parked due to weight limit are waiting.
    weight_increased: Notify,
    // Number of tasks that are not parked due to weight limit.
    eligible_tasks: AtomicUsize,
    // Hands out permits to tasks, limiting the amount of total parallelism.
    task_permits: Semaphore,
    // Overall amount of tasks that are still running or parked.
    remaining_tasks: AtomicUsize,
}

struct TaskPermit<'a> {
    scheduler: &'a Scheduler,
    permit: SemaphorePermit<'a>,
}

impl Scheduler {

    /// Create a new scheduler that limits the maximum parallelism to the given `fork_limit`
    /// and initially parks threads with more than `max_weight`.
    pub fn new(tasks: usize, fork_limit: usize, max_weight: usize) -> Scheduler {
        Scheduler {
            fork_limit,
            weight_limit: AtomicUsize::new(max_weight),
            weight_increased: Notify::new(),
            eligible_tasks: AtomicUsize::new(tasks),
            task_permits: Semaphore::new(fork_limit),
            remaining_tasks: AtomicUsize::new(tasks),
        }
    }

    /// Called by a task that completed. This also releases its task permit.
    pub fn finish_task<'a>(&'a self, permit: TaskPermit<'a>) {
        let eligible_tasks = self.remaining_tasks.fetch_sub(1, Ordering::SeqCst);
        // The task must have been eligible since it is running right now.
        let remaining_tasks = self.eligible_tasks.fetch_sub(1, Ordering::SeqCst);

        // At this point, we have to also test the same thing as in `acquire_permit` to ensure
        // that finished tasks do not artificially block existing tasks from running.
        if remaining_tasks <= self.fork_limit || eligible_tasks <= self.fork_limit {
            // TODO: Checked overflow.
            let weight_limit = self.weight_limit.load(Ordering::SeqCst);
            let new_weight_limit = min(2 * weight_limit, weight_limit + 10_000_000);
            if self.weight_limit.compare_exchange(weight_limit, new_weight_limit, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                self.weight_increased.notify_waiters();
            }
        }

        println!("Remaining: {}", remaining_tasks);

        drop(permit);
    }

    /// Acquire a work permit form the scheduler. The permit is automatically rewoked when
    /// the returned object is dropped.
    pub async fn acquire_permit<'a>(&'a self, current_weight: usize) -> TaskPermit<'a> {
        loop {
            let weight_limit = self.weight_limit.load(Ordering::SeqCst);
            if current_weight <= weight_limit {
                // The process is eligible for work, but needs a permit to ensure there
                // aren't too many threads hammering the CPU resources.
                let semaphore_permit = self.task_permits.acquire().await;
                let semaphore_permit = semaphore_permit.unwrap();
                return TaskPermit {
                    scheduler: self,
                    permit: semaphore_permit,
                }
            } else {
                // The process is not eligible for work now. We need to park it.
                // First, remove it from eligible processes.
                let eligible_tasks = self.eligible_tasks.load(Ordering::SeqCst);
                let remaining_tasks = self.remaining_tasks.load(Ordering::SeqCst);
                if remaining_tasks <= self.fork_limit || eligible_tasks <= self.fork_limit {
                    // We need to increase the weight limit for all processes. This is because
                    // we either don't have enough tasks to saturate the CPU, so we might as well
                    // just run them all, or the number of eligible tasks is smaller than
                    // the available level of parallelism, so we need to release some tasks to
                    // make up for this.

                    // We intentionally compare to the old value to ensure that only one process
                    // can truly increase the weight limit. Everyone else will just continue
                    // the loop and try again (either seeing a new result, or attempting again
                    // to set a new limit).

                    // TODO: Checked overflow.
                    let new_weight_limit = min(2 * weight_limit, weight_limit + 10_000_000);
                    if self.weight_limit.compare_exchange(weight_limit, new_weight_limit, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                        self.weight_increased.notify_waiters();
                    }
                } else {
                    // There is still enough eligible tasks to keep the CPU saturated.
                    // We should mark ourselves as non-eligible and wait to be notified.
                    self.eligible_tasks.fetch_sub(1, Ordering::SeqCst);
                    self.weight_increased.notified().await;
                    // Once we wake up, we mark ourselves as eligible again:
                    self.eligible_tasks.fetch_add(1, Ordering::SeqCst);
                }
            }

        }
    }

}

pub async fn schedule_reductions(
    stg: SymbolicAsyncGraph,
    fork_limit: usize,
) -> GraphColoredVertices {
    let stg = Arc::new(stg);
    let scheduler = Arc::new(Scheduler::new(stg.as_network().num_vars(), fork_limit, 10_000));
    let timestamp = Arc::new(AtomicUsize::new(0));
    let universe = Arc::new(RwLock::new(stg.mk_unit_colored_vertices()));

    let running = Arc::new(AtomicUsize::new(0));

    let mut processes = stg.as_network().variables()
        .map(|var| {
            let stg = stg.clone();
            let scheduler = scheduler.clone();
            let timestamp = timestamp.clone();
            let universe = universe.clone();
            let running = running.clone();
            tokio::spawn(async move {
                let mut process = ItgrProcess::new(&stg, var);
                let mut last_timestamp = 0;
                loop {
                    // Acquire permit from scheduler.
                    let permit = scheduler.acquire_permit(process.weight()).await;

                    /*{
                        let running = running.fetch_add(1, Ordering::SeqCst);
                        println!("Running: {} / {}", running + 1, fork_limit);
                    }*/

                    // Check if we need to update universe.
                    let current_timestamp = timestamp.load(Ordering::SeqCst);
                    if current_timestamp > last_timestamp {
                        last_timestamp = current_timestamp;
                        let universe = universe.read().await;
                        process.restrict(&universe, last_timestamp);
                        drop(universe);
                    }

                    // Perform a process step.
                    let (done, to_remove) = process.step().await;

                    // If necessary, remove states from the universe.
                    if let Some(to_remove) = to_remove {
                        let mut universe = universe.write().await;
                        *universe = universe.minus(&to_remove);
                        println!("Remaining universe: {}", universe.approx_cardinality());
                        timestamp.fetch_add(1, Ordering::SeqCst);
                        drop(universe);
                    }

                    // If done, signal to scheduler, otherwise drop permit and request new one.
                    if done {
                        scheduler.finish_task(permit);
                        /*{
                            let running = running.fetch_sub(1, Ordering::SeqCst);
                            println!("[Finished] Running: {} / {}", running + 1, fork_limit);
                        }*/
                        return;
                    } else {
                        drop(permit);
                        /*{
                            let running = running.fetch_sub(1, Ordering::SeqCst);
                            println!("Running: {} / {}", running + 1, fork_limit);
                        }*/
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    futures::future::join_all(processes.into_iter()).await;

    let result = universe.read().await;
    return result.clone();
    /*let mut futures = FuturesUnordered::new();

    // TODO: Add atomic counter to track actually running tasks.

    let count = Arc::new(AtomicUsize::new(0));

    while !processes.is_empty() || !futures.is_empty() {
        // Put processes with the smallest weight at the end of the vector.
        processes.sort_by_cached_key(|p| -(p.weight() as isize));

        while futures.len() < fork_limit && processes.len() > 0 {
            let mut process = processes.pop().unwrap();

            // Before queueing up the process, check if it has all the latest updates.
            if process.last_timestamp() < universe.1 {
                process.restrict(&universe.0, universe.1);
            }

            let count = count.clone();

            futures.push(tokio::spawn(async move {
                let total = count.fetch_add(1, Ordering::SeqCst);
                println!("Running: {}", total + 1);
                let start = SystemTime::now();
                let mut iter = 0;
                loop {
                    iter += 1;
                    let (done, to_remove) = process.step().await;
                    if done || to_remove.is_some() || start.elapsed().unwrap() > Duration::from_secs(5) {
                        println!("Task completed {} iterations.", iter);
                        let total = count.fetch_sub(1, Ordering::SeqCst);
                        println!("Running: {}", total - 1);
                        return (done, to_remove, process)
                    }
                }
            }));
        }

        let result = futures.next().await.unwrap();
        let (done, to_remove, process) = result.unwrap();

        if let Some(to_remove) = to_remove {
            universe.0 = universe.0.minus(&to_remove);
            universe.1 = universe.1 + 1;
        }

        if !done {
            processes.push(process);
        }

        println!("Remaining: {} + {}", processes.len(), futures.len());
    }


    universe.0*/
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