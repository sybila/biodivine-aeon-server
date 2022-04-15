use biodivine_lib_param_bn::{BooleanNetwork, RegulatoryGraph};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let network = args
        .get(1)
        .expect("Expected network path as first argument.");
    let network = std::fs::read_to_string(network.as_str()).unwrap();
    let network = BooleanNetwork::try_from_bnet(network.as_str()).unwrap();

    let variables = network
        .variables()
        .map(|it| network.get_variable_name(it).clone())
        .collect::<Vec<_>>();

    let mut rg = RegulatoryGraph::new(variables);

    for reg in network.as_graph().regulations() {
        rg.add_regulation(
            network.get_variable_name(reg.get_regulator()),
            network.get_variable_name(reg.get_target()),
            false,
            None,
        )
        .unwrap();
    }

    println!("{}", BooleanNetwork::new(rg).to_string());
}
