use std::convert::TryFrom;
use biodivine_lib_bdd::Bdd;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::BooleanNetwork;
use biodivine_lib_param_bn::symbolic_async_graph::SymbolicAsyncGraph;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let core_network = args[1].clone();
    let ensemble = args[2].clone();

    let core_bn = {
        let text = std::fs::read_to_string(core_network.as_str()).unwrap();
        BooleanNetwork::try_from(text.as_str()).unwrap()
    };

    //println!("Core BN size: {}", core_bn.num_vars());
    let stg = SymbolicAsyncGraph::new(core_bn.clone()).unwrap();

    //println!("BDD vars: {}", stg.symbolic_context().bdd_variable_set().num_vars());
    /*println!(
        "Core STG size: {}",
        stg.mk_unit_colored_vertices().approx_cardinality()
    );*/

    let mut colors = stg.mk_empty_colors();
    let dir_files = std::fs::read_dir(ensemble.as_str()).unwrap();
    for dir in dir_files {
        let dir = dir.unwrap();
        let file = dir.file_name();
        let file_name = file.to_str().unwrap();
        if file_name.ends_with(".bnet") {
            let string = std::fs::read_to_string(&dir.path()).unwrap();
            let sample = BooleanNetwork::try_from_bnet(string.as_str()).unwrap();
            let sample_string = sample.to_string().replace("-?", "-??");
            let sample = BooleanNetwork::try_from(sample_string.as_str()).unwrap();
            //println!("Read sample {:?}", file_name);
            let network_colors = stg.mk_subnetwork_colors(&sample).unwrap();
            colors = colors.union(&network_colors);
            //println!("Colors: {}", colors.as_bdd().size());
        }
    }

    let check = Bdd::from_string(colors.as_bdd().to_string().as_str());
    println!("Check: {}", check.size());
    println!("{}", colors.as_bdd().to_string());

    /*let colors_bdd = colors.as_bdd().clone();
    let all_bdd_vars = stg.symbolic_context().bdd_variable_set().variables();
    for var in core_bn.variables() {
        let table = stg.symbolic_context().get_implicit_function_table(var);
        let bdd_vars = table.into_iter().map(|(_, var)| var).collect::<Vec<_>>();

        let eliminate_vars = all_bdd_vars
            .iter()
            .cloned()
            .filter(|it| !bdd_vars.contains(it))
            .collect::<Vec<_>>();

        let function_bdd = colors_bdd.project(&eliminate_vars);
        println!(
            "BDD for function {:?}: {}. Is singleton? {}",
            core_bn.get_variable_name(var),
            function_bdd.size(),
            function_bdd.is_clause()
        );
        /*println!(
            "{}",
            function_bdd.to_dot_string(stg.symbolic_context().bdd_variable_set(), true)
        );*/
    }*/
}