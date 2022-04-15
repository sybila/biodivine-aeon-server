use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use std::convert::TryFrom;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let core_network = args
        .get(1)
        .expect("Expected path to the network sketch as the first argument.");
    let ensemble = args
        .get(2)
        .expect("Expected path to ensemble folder as the second argument.");

    let core_bn = {
        let text = std::fs::read_to_string(core_network.as_str()).unwrap();
        BooleanNetwork::try_from(text.as_str()).unwrap()
    };

    let stg = SymbolicAsyncGraph::new(core_bn.clone()).unwrap();

    let mut colors = stg.mk_empty_colors();
    let dir_files = std::fs::read_dir(ensemble.as_str()).unwrap();
    for dir in dir_files {
        let dir = dir.unwrap();
        let file = dir.file_name();
        let file_name = file.to_str().unwrap();
        let sample = if file_name.ends_with(".bnet") {
            let string = std::fs::read_to_string(&dir.path()).unwrap();
            BooleanNetwork::try_from_bnet(string.as_str()).unwrap()
        } else if file_name.ends_with(".aeon") {
            let string = std::fs::read_to_string(&dir.path()).unwrap();
            BooleanNetwork::try_from(string.as_str()).unwrap()
        } else if file_name.ends_with(".sbml") {
            let string = std::fs::read_to_string(&dir.path()).unwrap();
            BooleanNetwork::try_from_sbml(string.as_str()).unwrap().0
        } else {
            eprintln!("Skipping {}.", file_name);
            continue;
        };

        eprintln!("Loaded sample from {}.", file_name);

        // Replace regulations with non-observable ones.
        let sample_string = sample.to_string().replace("-?", "-??");
        let sample = BooleanNetwork::try_from(sample_string.as_str()).unwrap();

        let network_colors = stg.mk_subnetwork_colors(&sample).unwrap();
        colors = colors.union(&network_colors);
    }

    println!("{}", colors.as_bdd().to_string());
}
