use biodivine_lib_param_bn::{
    BinaryOp, BooleanNetwork, FnUpdate, Monotonicity, RegulatoryGraph, VariableId,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const V_COUNT: usize = 1000;
const R_COUNT: usize = 3000;

fn main() {
    //let mut random = StdRng::seed_from_u64(1234567890);
    let mut random = StdRng::seed_from_u64(123456789);

    let current_dir = std::env::current_dir().unwrap();
    let aeon_benchmarks = current_dir.join("random_aeon_models_1000");
    if !aeon_benchmarks.exists() {
        std::fs::create_dir_all(&aeon_benchmarks).unwrap();
    }

    for i_model in 1..101 {
        let variables = (0..V_COUNT).map(|i| format!("x{}", i)).collect::<Vec<_>>();
        let mut rg = RegulatoryGraph::new(variables.clone());
        let mut remaining_regs = 0;
        // Each variable must have in/out-degree at least one...
        for source in rg.variables() {
            if !rg.regulators(source).is_empty() || !rg.targets(source).is_empty() {
                continue;
            }
            let source: usize = source.into();
            let target: usize = random.gen_range(0..V_COUNT);
            let monotonicity = if random.gen_bool(0.7) {
                Monotonicity::Activation
            } else {
                Monotonicity::Inhibition
            };
            if random.gen_bool(0.5) {
                rg.add_regulation(
                    &variables[source],
                    &variables[target],
                    true,
                    Some(monotonicity),
                )
                .unwrap();
            } else {
                rg.add_regulation(
                    &variables[target],
                    &variables[source],
                    true,
                    Some(monotonicity),
                )
                .unwrap();
            }
            remaining_regs += 1;
        }
        // Each variable must have in-degree at least one...
        /*for target in rg.variables() {
            if rg.regulators(target).len() > 0 { continue; }
            let source: usize = random.gen_range(0..V_COUNT);
            let target: usize = target.into();
            let monotonicity = if random.gen_bool(0.7) { Monotonicity::Activation } else { Monotonicity::Inhibition };
            rg.add_regulation(&variables[source], &variables[target], true, Some(monotonicity)).unwrap();
            remaining_regs += 1;
        }*/
        // Finally, the whole thing must be one SCC
        /*while rg.components().len() > 1 {
            let components = rg.components();
            let c1 = &components[0];
            let c2 = &components[1];
            let i_source = random.gen_range(0..c1.len());
            let t_source = random.gen_range(0..c2.len());
            let source: usize = c1.iter().skip(i_source).next().unwrap().clone().into();
            let target: usize = c2.iter().skip(t_source).next().unwrap().clone().into();
            if rg.find_regulation(VariableId::from(source), VariableId::from(target)).is_none() {
                let monotonicity = if random.gen_bool(0.7) { Monotonicity::Activation } else { Monotonicity::Inhibition };
                rg.add_regulation(&variables[source], &variables[target], true, Some(monotonicity)).unwrap();
                remaining_regs += 1;
            }
            let i_source = random.gen_range(0..c2.len());
            let t_source = random.gen_range(0..c1.len());
            let source: usize = c2.iter().skip(i_source).next().unwrap().clone().into();
            let target: usize = c1.iter().skip(t_source).next().unwrap().clone().into();
            if rg.find_regulation(VariableId::from(source), VariableId::from(target)).is_none() {
                let monotonicity = if random.gen_bool(0.7) { Monotonicity::Activation } else { Monotonicity::Inhibition };
                rg.add_regulation(&variables[source], &variables[target], true, Some(monotonicity)).unwrap();
                remaining_regs += 1;
            }
        }*/
        // Rest is truly random
        while remaining_regs <= R_COUNT {
            let source = random.gen_range(0..V_COUNT);
            let target = random.gen_range(0..V_COUNT);
            if rg
                .find_regulation(VariableId::from(source), VariableId::from(target))
                .is_none()
            {
                let monotonicity = if random.gen_bool(0.7) {
                    Monotonicity::Activation
                } else {
                    Monotonicity::Inhibition
                };
                rg.add_regulation(
                    &variables[source],
                    &variables[target],
                    true,
                    Some(monotonicity),
                )
                .unwrap();
                remaining_regs += 1;
            }
        }

        let max_degree_var = rg
            .variables()
            .max_by_key(|v| rg.regulators(*v).len())
            .unwrap();
        let max_degree = rg.regulators(max_degree_var).len();
        eprintln!("Generated network has max degree {}", max_degree);

        let mut bn = BooleanNetwork::new(rg.clone());

        for v in bn.variables() {
            let regulators = bn.regulators(v);
            if regulators.is_empty() {
                bn.add_update_function(v, FnUpdate::Const(random.gen_bool(0.5)))
                    .unwrap();
            } else {
                let r = regulators[0];
                let fst_is_activation = rg.find_regulation(r, v).unwrap().get_monotonicity()
                    == Some(Monotonicity::Activation);
                let mut fn_update = if fst_is_activation {
                    FnUpdate::Var(r)
                } else {
                    FnUpdate::Not(Box::new(FnUpdate::Var(r)))
                };
                for r in regulators.iter().cloned().skip(1) {
                    let op = if random.gen_bool(0.5) {
                        BinaryOp::And
                    } else {
                        BinaryOp::Or
                    };
                    let is_activation = rg.find_regulation(r, v).unwrap().get_monotonicity()
                        == Some(Monotonicity::Activation);
                    let var = if is_activation {
                        FnUpdate::Var(r)
                    } else {
                        FnUpdate::Not(Box::new(FnUpdate::Var(r)))
                    };
                    fn_update = FnUpdate::Binary(op, Box::new(fn_update), Box::new(var));
                }
                bn.add_update_function(v, fn_update).unwrap();
            }
        }

        let aeon_file = aeon_benchmarks.join(&format!("{}_{}_{}.aeon", i_model, V_COUNT, R_COUNT));
        std::fs::write(aeon_file, bn.to_string()).unwrap();
        println!("{} generated...", i_model);
    }
}
