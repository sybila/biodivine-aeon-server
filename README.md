# Aeon Compute Engine

This is a compute engine for the Aeon tool (http://biodivine.fi.muni.cz/aeon/).

If you downloaded a pre-built binary, you can just run it to start the engine. The binary will output
current address and port on which the compute engine is running. Later, when performing actions, there
will some additional logging output. The engine will automatically run on localhost:8000 which is the 
default setting for the online client. Hence once the engine is running, you should be able to 
connect immediately.

If for some reason, you can't run the engine on `localhost:8000`, you can configure the address and port
using environmental variables `AEON_ADDR` and `AEON_PORT`. Then you have to input this address and port 
also into the online client when connecting. 

To configure the amount of parallel workers that are used by the binary, you can use
the environmental variable `RAYON_NUM_THREADS` (by default, the number of workers 
will be 2x the available CPUs, but not all workers are necessarily used during all
computations). 

Note that some computations may require substantial amount of RAM.

More information about the operation of Aeon can be found in the official manual at
http://biodivine.fi.muni.cz/aeon/manual.pdf

### Building from source
To build the engine from source, you need a nightly Rust toolchain. Then you can simply run `cargo build --release`
in project root directory. The binary will be located in `target/release/biodivine-aeon-server`.

## Standalone experiments

To run analysis of a single model in command line, you can use the following command:

```
cargo run --release --bin experiment < path/to/model.aeon
``` 

As the model path, you can use one of the `.aeon` models provided 
in the `benchmarks` directory (note that some might require non-trivial
time and memory).

If you have [cargo make](https://github.com/sagiegurari/cargo-make) installed, you can
also run the same command more easily with:

```
cargo make experiment < path/to/model.aeon
```

Here are expected run times for some of the benchmark models (measured on
a 32-core workstation with 64GB of memory):

| Model File                                   | Time (1CPU) | Time (32CPU) |
|----------------------------------------------|-------------|--------------|
| Asymmetric Cell Division (A)_parametric.aeon | 0:05.62     | 0:03.39      |
| Budding yeast (Orlando)_parametric.aeon      | 0:35.22     | 0:02.93      |
| TCR signalisation_parametric.aeon            | 0:26.61     | 0:04.42      |
| Drosophila cell cycle_parametric.aeon        | 27:48.1     | 1:42.28      |
| Fission Yeast Cell Cycle_parametric.aeon     | 25:20.9     | 4:00.29      |
| Mammalian Cell Cycle_parametric.aeon         | 38:39.6     | 8:02.14      |
| Budding yeast (Irons)_parametric.aeon        | timeout     | 52:28.1      |