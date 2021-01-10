use biodivine_lib_param_bn::{BooleanNetwork, FnUpdate, BinaryOp};
use std::convert::TryFrom;

fn main() {
    let current_dir = std::env::current_dir().unwrap();
    let bnet_benchmarks = current_dir.join("random_bnet_models_1000");
    if !bnet_benchmarks.exists() {
        std::fs::create_dir_all(&bnet_benchmarks).unwrap();
    }

    let benchmarks = std::fs::read_dir("./random_aeon_models_1000").unwrap();
    for bench_dir in benchmarks.into_iter().map(|it| it.unwrap()) {
        let bench_name = bench_dir.file_name().to_str().unwrap().to_string();
        let model_path = bench_dir.path();
        let model_string = std::fs::read_to_string(model_path);
        if model_string.is_err() { continue; }
        let model_string = model_string.unwrap();
        let r = BooleanNetwork::try_from(model_string.as_str());
        if r.is_err() { continue; }
        let model = r.unwrap();
        let bnet_file = bnet_benchmarks.join(&format!("{}.bnet", bench_name));
        std::fs::write(bnet_file, network_to_bnet(&model)).unwrap();
    }
}


fn network_to_bnet(network: &BooleanNetwork) -> String {
    let mut model = format!("targets,factors\n");
    for v in network.variables() {
        let v_id: usize = v.into();
        let line = format!("v{}, {}\n", v_id, fnupdate_to_bnet_string(network.get_update_function(v).as_ref().unwrap()));
        model.push_str(&line);
    }
    model
}

fn fnupdate_to_bnet_string(fn_update: &FnUpdate) -> String {
    match fn_update {
        FnUpdate::Param(_, _) => panic!("Parameters not allowed."),
        FnUpdate::Const(value) => {
            if *value {  // There is always v1
                format!("v1 | !v1",)
            } else {
                format!("v1 & !v1",)
            }
        }
        FnUpdate::Var(id) => {
            let id: usize = (*id).into();
            format!("v{}", id)
        }
        FnUpdate::Not(inner) => format!("!{}", fnupdate_to_bnet_string(inner)),
        FnUpdate::Binary(op, l, r) => {
            let left = fnupdate_to_bnet_string(l);
            let right = fnupdate_to_bnet_string(r);
            let op = match *op {
                BinaryOp::And => "&",
                BinaryOp::Or => "|",
                _ => panic!("{:?} not supported.", op),
            };
            format!("({} {} {})", left, op, right)
        }
    }
}

