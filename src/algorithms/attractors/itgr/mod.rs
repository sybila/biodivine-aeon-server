use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::{GraphColoredVertices, SymbolicAsyncGraph};
use tokio::sync::{Mutex, MutexGuard, RwLock, Semaphore, SemaphorePermit};
use crate::algorithms::attractors::itgr::itgr_process::ItgrProcess;
use tokio::sync::oneshot;

mod reachability_process;
mod itgr_process;

struct Scheduler {
    // The maximal amount of parallelism allowed in this scheduler.
    fork_limit: usize,
    // Hands out permits to tasks, limiting the amount of total parallelism.
    task_permits: Semaphore,
    // Tracks the maximum weight of a task that is allowed to execute right now.
    // Note that the limit can only increase.
    weight_limit: AtomicUsize,
    // Tracks the number of all remaining tasks and a list of tasks waiting to be resumed.
    //  > Note that this is slightly cleaner than using `Notify` because we'd have to put that
    //  > into an Arc to make sure sender and receiver sides do not drop it prematurely.
    wait_list: Mutex<(usize, Vec<oneshot::Sender<()>>)>,
}

struct TaskPermit<'a> {
    _scheduler: &'a Scheduler,
    _permit: SemaphorePermit<'a>,
    weight_limit: usize,
}

impl<'a> TaskPermit<'a> {

    pub fn is_valid_for(&self, weight: usize) -> bool {
        weight <= self.weight_limit
    }

}

impl Scheduler {

    /// Create a new scheduler that limits the maximum parallelism to the given `fork_limit`
    /// and initially parks threads with more than `max_weight`.
    pub fn new(tasks: usize, fork_limit: usize, max_weight: usize) -> Scheduler {
        Scheduler {
            fork_limit,
            weight_limit: AtomicUsize::new(max_weight),
            task_permits: Semaphore::new(fork_limit),
            wait_list: Mutex::new((tasks, Vec::new())),
        }
    }

    pub async fn finish_task<'a>(&self) {
        let mut wait_list = self.wait_list.lock().await;
        wait_list.0 -= 1;   // Decrease the number of tasks.
        println!("Remaining tasks: {}.", wait_list.0);

        // At this point, the occupancy of the CPU might decrease. If necessary, we have to
        // increase the weight limit to start running new tasks.
        if wait_list.0 - wait_list.1.len() < self.fork_limit {
            self.increase_weight_limit_and_notify(wait_list);
        }
    }

    fn increase_weight_limit_and_notify(&self, mut guard: MutexGuard<(usize, Vec<oneshot::Sender<()>>)>) {
        // 10M seems like a reasonable upper bound for exponential growth at the moment.
        // But we might need to increase it at some point.
        let weight_limit = self.weight_limit.load(Ordering::SeqCst);
        let new_weight_limit = min(2 * weight_limit, weight_limit + 10_000_000);
        self.weight_limit.store(new_weight_limit, Ordering::SeqCst);

        // Now we can notify everyone that was waiting for a weight increase.
        while let Some(send) = guard.1.pop() {
            send.send(()).unwrap();
        }
    }

    pub async fn acquire_permit<'a>(&'a self, weight: usize) -> TaskPermit<'a> {
        loop {
            let weight_limit = self.weight_limit.load(Ordering::SeqCst);
            if weight <= weight_limit {
                // If we are still within the weight limit, we can just wait for the semaphore
                // and return a permit.
                let semaphore_permit = self.task_permits.acquire().await;
                let semaphore_permit = semaphore_permit.unwrap();
                return TaskPermit {
                    _permit: semaphore_permit,
                    _scheduler: &self,
                    weight_limit,
                }
            } else {
                let mut wait_list = self.wait_list.lock().await;

                // We have to reload the weight limit because it might have been increased
                // while we waited for the mutex to lock. Since we are only increasing it
                // when the mutex is locked, nobody should mess with it until we drop
                // the `wait_list` guard.
                let observed_weight_limit = weight_limit;
                let weight_limit = self.weight_limit.load(Ordering::SeqCst);
                if observed_weight_limit != weight_limit {
                    continue;
                }

                if wait_list.0 - wait_list.1.len() < self.fork_limit {
                    // The number of total tasks that are not waiting is smaller than the number
                    // of tasks that are allowed to run under the current limit. Let's increase
                    // the limit and notify everyone.
                    self.increase_weight_limit_and_notify(wait_list);
                } else {
                    // There is still a sufficient amount of eligible tasks to saturate the CPU.
                    // We can just add ourselves to the wait list, drop the lock and wait for
                    // someone to send us a restart signal.
                    let (send, receive) = oneshot::channel::<()>();
                    wait_list.1.push(send);
                    println!("Wait list: {}.", wait_list.1.len());
                    // We need to drop the guard here to avoid blocking the list while we wait.
                    // In other branches, this happens automatically as the guard gets out of scope.
                    drop(wait_list);
                    receive.await.unwrap();
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

                    while permit.is_valid_for(process.weight()) {

                        // Check if we need to update the universe.
                        let current_timestamp = timestamp.load(Ordering::SeqCst);
                        if current_timestamp > last_timestamp {
                            last_timestamp = current_timestamp;
                            // Clone the universe into out process while keeping the guard for
                            // as little time as possible.
                            let universe = {
                                let guard = universe.read().await;
                                guard.clone()
                            };
                            // Because this is probably going to take some time...
                            process.restrict(&universe, last_timestamp);
                        }

                        // Perform a process step.
                        let (done, to_remove) = process.step().await;

                        // If necessary, remove states from the universe.
                        if let Some(to_remove) = to_remove {
                            if !to_remove.is_empty() {
                                let mut universe = universe.write().await;
                                *universe = universe.minus(&to_remove);
                                println!("Remaining universe: {}", universe.approx_cardinality());
                                timestamp.fetch_add(1, Ordering::SeqCst);
                                drop(universe);
                            }
                        }

                        // If done, signal to scheduler, otherwise drop permit and request new one.
                        if done {
                            scheduler.finish_task().await;
                            /*{
                                let running = running.fetch_sub(1, Ordering::SeqCst);
                                println!("[Finished] Running: {} / {}", running + 1, fork_limit);
                            }*/
                            return;
                        }
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