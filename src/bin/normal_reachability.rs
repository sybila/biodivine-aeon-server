use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;
use std::time::SystemTime;
use std::io::Read;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::BooleanNetwork;
use std::convert::TryFrom;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    let names: Vec<_> = model
        .variables()
        .map(|id| model.get_variable_name(id))
        .collect();
    println!("Model loaded...");
    println!("{} variables: {:?}", model.num_vars(), names);

    let graph = SymbolicAsyncGraph::new(model).unwrap();

    println!("Asynchronous graph ready...");
    println!(
        "Admissible parametrisations: {}",
        graph.unit_colors().approx_cardinality()
    );
    println!(
        "State space: {}",
        graph.unit_colored_vertices().approx_cardinality()
    );

    let mut universe = graph.mk_unit_colored_vertices();
    while !universe.is_empty() {
        let mut vertices = universe.pick_vertex();
        loop {
            let mut stop = true;
            for v in graph.as_network().variables() {
                let var_post = graph.var_post(v, &vertices);
                stop = stop && var_post.is_subset(&vertices);
                let size_before = vertices.as_bdd().size();
                vertices = vertices.union(&var_post);
                println!("Operation: {} | {} = {}", size_before, var_post.as_bdd().size(), vertices.as_bdd().size());
            }
            if stop {
                break;
            }
            /*let post = graph.post(&vertices).minus(&vertices).intersect(&universe);
            if post.is_empty() {
                break;
            }
            let size_before = vertices.as_bdd().size();
            vertices = vertices.union(&post);*/
        }
        println!("Iteration removed: {}", vertices.approx_cardinality());
        universe = universe.minus(&vertices);
    }

}