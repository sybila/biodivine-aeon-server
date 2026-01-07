# Aeon Compute Engine

A compute server for [AEON](http://biodivine.fi.muni.cz/aeon/) - a tool for analyzing parametrized Boolean networks. The server provides a REST API for attractor computation, bifurcation analysis, stability analysis, and control computation using efficient symbolic algorithms based on Binary Decision Diagrams (BDDs).

## Features

- **Attractor Computation**: Compute all asynchronous attractors of parametrized Boolean networks using the Xie-Beerel algorithm with ITGR reduction
- **Bifurcation Trees**: Build and explore decision trees that partition the parameter space based on attractor types
- **Stability Analysis**: Analyze variable stability (stable/unstable/switched) across different attractor behaviors
- **Control Computation**: Find minimal perturbations that achieve desired phenotype behaviors (permanent control)
- **Format Conversion**: Convert between Aeon format and SBML (Systems Biology Markup Language)
- **Witness Extraction**: Generate concrete network instantiations (witnesses) for specific parameter regions
- **Session Management**: Multi-client support with session-based state isolation
- **Progress Tracking**: Real-time progress updates and cancellation support for long-running computations

## Installation

### Pre-built Binaries

Pre-built binaries are available for Linux, macOS, and Windows. Download the appropriate binary for your platform from the [releases page](https://github.com/sybila/biodivine-aeon-server/releases).

 > **Windows/macOS Warning:** The binaries are not signed, meaning you will need to allow applications from
 > untrusted developers. This process can vary greatly depending on the OS version. For Windows, 
 > confirming the popup dialog that appears on the first start should be enough. For macOS, you may need to go to security 
 > settings *after* trying to open the application for the first time 
 > (alternatively, run `xattr -dr com.apple.quarantine path/to/biodivine-aeon-server` in the terminal).

### Building from Source

This project requires Rust. The code uses Rust edition 2024, which requires a recent Rust compiler (Rust 1.90.0 or later).

1. **Install [Rust](https://rust-lang.org/learn/get-started/)** (if not already installed):

3. **Build the project**:
   ```bash
   cargo build --release
   ```

The binary will be located at `target/release/biodivine-aeon-server`.

### Installing from git

You can also use `cargo` to directly install the binary on your system:

```bash
cargo install --git https://github.com/sybila/biodivine-aeon-server.git
```

## Usage

### Starting the Server

Simply run the binary to start the server:

```bash
./target/release/biodivine-aeon-server
```

Or build it and immediately run via cargo:

```bash
cargo run --release
```

By default, the server listens on `127.0.0.1:8000`. The server will print the address and port when it starts.

### Remote access

Since this is a normal HTTP server, you can also access it over the internet. However, due to CORS,
you won't be able to do that from the official HTTPS client. And you really should not expose the
HTTP server to open internet anyway.

*Currently, for remote deployments, we recommend running AEON server behind a reverse proxy like `nginx` 
or `cuddy`, implementing HTTPS and authentication at the level of the reverse proxy service.*

### Configuration

The server can be configured using environment variables:

- **`AEON_ADDR`**: Server address (default: `127.0.0.1`)
- **`AEON_PORT`**: Server port (default: `8000`). Alternatively, you can pass the port as the first command-line argument.

Example:
```bash
AEON_ADDR=0.0.0.0 AEON_PORT=9000 ./biodivine-aeon-server
```

### Connecting from the Aeon Client

Once the server is running, you can connect from the [Aeon web client](http://biodivine.fi.muni.cz/aeon/). The default address (`localhost:8000`) should work automatically. If you've configured a different address or port, enter it when connecting.

### API Endpoints

The server provides the following main API endpoints:

- **Computation Management**:
  - `POST /start_computation` - Start attractor computation for a model
  - `POST /cancel_computation` - Cancel a running computation
  - `GET /ping` - Check computation status
  - `GET /get_results` - Get attractor classification results

- **Bifurcation Trees**:
  - `GET /get_bifurcation_tree` - Get the current bifurcation tree
  - `GET /get_attributes/<node_id>` - Get attributes for a tree node
  - `POST /apply_attribute/<node_id>/<attribute_id>` - Apply an attribute to expand the tree
  - `POST /revert_decision/<node_id>` - Revert a decision node
  - `POST /auto_expand/<node_id>/<depth>` - Automatically expand a tree branch
  - `POST /apply_tree_precision/<precision>` - Set tree precision
  - `GET /get_tree_precision` - Get current tree precision

- **Stability Analysis**:
  - `GET /get_stability_data/<node_id>/<behaviour>` - Get stability data for a tree node
  - `GET /get_stability_witness/<node_id>/<behaviour>/<variable>/<vector>` - Get witness for stability vector
  - `GET /get_stability_attractors/<node_id>/<behaviour>/<variable>/<vector>` - Get attractors for stability vector

- **Witnesses and Attractors**:
  - `GET /get_witness/<class>` - Get witness network for a behavior class
  - `GET /get_attractors/<class>` - Get attractors for a behavior class
  - `GET /get_tree_witness/<node_id>` - Get witness for a tree node
  - `GET /get_tree_attractors/<node_id>` - Get attractors for a tree node

- **Control Computation**:
  - `POST /start_control_computation/<oscillation>/<min_robustness>/<max_size>/<result_count>` - Start control computation
  - `POST /cancel_control_computation` - Cancel control computation
  - `GET /get_control_computation_status` - Get control computation status
  - `GET /get_control_results` - Get control computation results
  - `GET /get_control_stats` - Get control computation statistics

- **Format Conversion**:
  - `POST /sbml_to_aeon` - Convert SBML to Aeon format
  - `POST /aeon_to_sbml` - Convert Aeon to SBML format
  - `POST /aeon_to_sbml_instantiated` - Convert Aeon to instantiated SBML (witness model)

- **Utility**:
  - `POST /check_update_function` - Validate and get cardinality of an update function

All endpoints return JSON responses with either `{"status": true, "result": ...}` for success or `{"status": false, "message": ...}` for errors.

### Session Management

The server supports multiple concurrent clients through session keys. Include a `x-session-key` header in your requests to isolate state between different clients. If not provided, requests default to a global session.

## Standalone Command-Line Tools

The project includes several standalone binaries for batch processing:

### Attractor Analysis (`experiment`)

Run attractor analysis on a model from the command line:

```bash
cargo run --release --bin experiment < path/to/model.aeon
```

This will perform a complete attractor computation and print the results, including:
- Model information (variables, parameterization set size, state space size)
- Discovered attractors with their cardinalities
- Behavior class classification
- Total computation time

### Other Utilities

- **`benchmark_filter`**: Process benchmark models and convert them to Aeon format
- **`sink_state_enumerator`**: Enumerate sink states (stable attractors)

## Documentation

- **User Manual**: Comprehensive documentation is available at [https://biodivine.fi.muni.cz/aeon/manual/v0.5.0/index.html](https://biodivine.fi.muni.cz/aeon/manual/v0.5.0/index.html)
- **API Documentation**: Generate API documentation with `cargo doc --open`

## Development

The codebase is organized into several modules:

- **`bdt/`**: Bifurcation Decision Tree implementation
- **`scc/`**: Strongly Connected Component classification and stability analysis
- **`control/`**: Control computation functionality
- **`util/`**: Utility functions and helpers

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
