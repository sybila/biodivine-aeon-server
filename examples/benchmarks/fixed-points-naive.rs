use biodivine_aeon_server::algorithms::{inline_network_inputs, FixedPoints};
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;

#[tokio::main]
async fn main() {
    let model = load_network_from_args();
    let model = inline_network_inputs(model);
    let stg = SymbolicAsyncGraph::new(model).unwrap();
    let all_vertices = stg.mk_unit_colored_vertices();
    let all_variables = stg.as_network().variables().collect::<Vec<_>>();
    let fixed_points = FixedPoints::naive(&stg, &all_vertices, &all_variables).await;
    println!(
        "Compute {} fixed-points with symbolic size {}.",
        fixed_points.approx_cardinality(),
        fixed_points.symbolic_size()
    );
}

fn load_network_from_args() -> BooleanNetwork {
    let args = std::env::args().collect::<Vec<_>>();
    BooleanNetwork::try_from(std::fs::read_to_string(&args[1]).unwrap().as_str()).unwrap()
}
