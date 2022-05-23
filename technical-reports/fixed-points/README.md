# Fixed-points

Fixed points, or single-state attractors (sinks) represent one of the most 
basic properties that we can observe in a Boolean network. They are 
particularly interesting because they are shared across every BN semantics.

Here, we compare three approaches to computing BN fixed-points:

 1. Naive algorithm: Iterates through network variables in their declared 
    order and removes all states that can perform a transition using this 
    variable. Problem of this algorithm is that the intermediate 
    representation can often explode during this process, even if the result 
    is compact.
 2. Greedy algorithm: This algorithm improves on the naive one by 
    prioritising sets of vertices that have the smallest symbolic 
    representation. This is not helpful if the result is not compact, 
    but at least it tends to avoid the problem of exploding intermediate 
    representations.
 3. Greedy parallel algorithm: A version of the greedy algorithm where we 
    also parallelize most performed symbolic operations. The algorithm must 
    still synchronize when greedily picking new set of transitions. However, 
    at least the partial operations in each step can be performed in parallel.

### Running

First (starting in project root), compile the binaries (including logging), and 
copy them to the artifact folder:

```bash
cargo build --release --features log-progress --example fixed-points-naive
cargo build --release --features log-progress --example fixed-points-greedy
cargo build --release --features log-progress --example fixed-points-greedy-parallel

cp ./target/release/examples/fixed-points-naive ./technical-reports/fixed-points/artefact
cp ./target/release/examples/fixed-points-greedy ./technical-reports/fixed-points/artefact
cp ./target/release/examples/fixed-points-greedy-parallel ./technical-reports/fixed-points/artefact
```

To run the individual benchmark runs, go to the `artefact` directory and use 
the following commands:

```bash
cd technical-reports/fixed-points/artefact

python3 ../../run.py 1h models ./fixed-points-naive 
python3 ../../run.py 1h models ./fixed-points-greedy
python3 ../../run.py 1h models ./fixed-points-greedy-parallel
```

The first run can be quite long as there might be multiple timeouts, the 
subsequent ones will be a bit faster since the algorithms are better. In 
the second case, probably expect one timeout, in the second case there 
should hopefully be no timeouts, but a few computations can take tens of 
minutes or more.

### Results

Below is a table of all experiments that required more than one second to 
complete. Importantly 6 naive benchmarks didn't finish within one hour. 
Meanwhile, when replaced with the greedy approach, everything finished, and 
generally also faster (`122.aeon` being the only exception). Subsequently, 
the parallel version was also faster than the greedy version, outperforming 
the naive algorithm reliably every time. 

Conditions: 
 - stock `Ryzen 7 5800X`, 
 - commit `a62bff651647576bf3cc2a64a2f3b86aac050642`,
 - Rust `1.60.0-stable`.

| Model | Naive   | Greedy  | Greedy-parallel |
|-------|---------|---------|-----------------|
| 1     | ---     | 17.2s   | 7.8s            |
| 2     | ---     | 0.2s    | 0.1s            |
| 4     | ---     | 2.4s    | 0.8s            |
| 16    | 66.8s   | 0.1s    | 0.1s            |
| 41    | 11.4s   | 0.5s    | 0.2s            |
| 50    | ---     | 102.5s  | 29.5s           |
| 51    | 1.5s    | 0.1s    | 0.1s            |
| 72    | 3.4s    | 0.1s    | 0.1s            |
| 78    | ---     | 38.1s   | 13.8s           |
| 80    | 23.3s   | 0.1s    | 0.1s            |
| 82    | 119.3s  | 0.2s    | 0.1s            |
| 83    | 1241.6s | 4.3s    | 1.5s            |
| 113   | ---     | 493.0s  | 208.1s          |
| 116   | 4.8s    | 0.2s    | 0.1s            |
| 118   | 22.9s   | 0.1s    | 0.1s            |
| 120   | 191.1s  | 39.1s   | 16.7s           |
| 122   | 2342.9s | 2992.3s | 1484.8s         |
| 124   | 276.7s  | 181.0s  | 103.2s          |
| 138   | 4.4s    | 0.1s    | 0.1s            |
| 143   | 156s    | 0.5s    | 0.1s            |

To conclude, the greedy approach makes the detection much more resilient to 
the choice of variable ordering. It generally scales with respect to the 
size of the result, while the naive approach may get stuck along the way 
with a large intermediate result. 

### Future work

*Minor:* There is probably a better way of running the greedy-parallel 
algorithm such that less synchronization is required. One option would be to 
always pick two smallest sets available and then compute their intersection 
in parallel, putting the result back into the shared pool. Not sure how 
resilient this option would be to the explosion in intermediate 
representation though, since the minimum is now just over the sets that 
nobody is working on at the moment.

*Major:* Second problem is with parametrised networks and the overall 
representation of the outputs. There are clear examples of cases where the 
resulting set of fixed-points is very large (`113.aeon` and `122.aeon`) and 
this issue cannot be entirely avoided. One obvious option is variable 
reordering. In this algorithm, it should be fairly trivial as all sets are 
stored in one place. Another option is to somehow group together similar 
parameter valuations and output multiple BDDs instead of a single one. The 
advantage of this approach is that it could be better parallelized. However, 
it will most certainly have more overhead than the approach using a single 
BDD. Maybe we can use the results compute for one parameter valuation to 
speed up computation for other valuations (or partial valuation).  