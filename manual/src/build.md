# Building Aeon

AEON lives in two Github repositories and is available under a permissive MIT license. First repository manages the web frontend, and the second one is responsible for the compute engine.

### Deploying the web frontend

If you want to run your own version of the AEON frontend, you simply need to download the latest source files from the [aeon-client](https://github.com/sybila/biodivine-aeon-client) repository (or download the sources for a particular release based on its tag). Then, you can either open the `index.html` file in your favourite browser, or deploy the whole directory to any webserver that is able to serve static content.

### Compiling the compute engine

To run your own version of the compute engine, you need to download the source files from the [aeon-server](https://github.com/sybila/biodivine-aeon-server) repository (again, you can either download the latest version from the `master` branch, or sources for a specific version based on its tag). To run or compile the compute engine, you will need the Rust nightly compiler. We recommend following the installation instructions on [rust-lang.org](https://www.rust-lang.org/). These will install the whole distribution into your local `~/.cargo` folder, hence they do not require any elevated privileges (as opposed to some OS package managers).

> Once you have completed the setup process, you need to switch from `stable` to the `nightly` compiler by executing: 
> ```
> rustup default nightly
> ```
 
Once you have the compiler ready, you can navigate to the source files you downloaded from Github, and execute 

```
cargo run --release
``` 

to immediately compile and start the compute engine. Alternatively, you can also run 

```
cargo build --release
```

which will generate the compute engine binary in `./target/release/biodivine-aeon-server`.

### Building this manual

This manual is available in the `aeon-server` repository in the `manual` directory, and is managed using the [mdBook](https://github.com/rust-lang/mdBook) tool. You can follow the instructions on their website to install `mdbook`. Once you have `mdbook` ready, you simply need to navigate to the `manual` folder and run `mdbook build` to write the output files into the `book` directory (from here, you can deploy them as a static website). Alternatively, you can run `mdbook serve` to open a local webserver at `http://localhost:3000` which serves the manual from your machine and automatically updates when you change its contents.