# Aeon Compute Engine

This is a compute engine for the Aeon tool (http://biodivine.fi.muni.cz/aeon/).

If you downloaded a pre-built binary, you can just run it to start the engine. The binary will output
current address and port on which the compute engine is running. Later, when performing actions, there
will some additional logging output. The engine will automatically run on localhost:8000 which is the 
default setting for the online client. Hence once the engine is running, you should be able to 
connect immediately.

If for some reason, you can't run the engine on localhost:8000, you can configure the address and port
using environmental variables AEON_ADDR and AEON_PORT. Then you have to input this address and port 
also into the online client when connecting. 

Note that some computations may require substantial amount of RAM.

More information about the operation of Aeon can be found in the official manual at
http://biodivine.fi.muni.cz/aeon/manual.pdf

### Building from source
To build the engine from source, you need a nightly Rust toolchain. Then you can simply run `cargo build --release`
in project root directory. The binary will be located in `target/release/biodivine-aeon-server`.
