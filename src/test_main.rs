use crate::scc::algo_components::components;
use crate::scc::Classifier;
use biodivine_lib_std::param_graph::{Graph, EvolutionOperator, Params};
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use biodivine_lib_param_bn::BooleanNetwork;
use std::convert::TryFrom;

pub fn run() {
    /*let model = BooleanNetwork::try_from("
        p53 -| DNA
        p53 -> M2C
        p53 -| M2N

        M2C -> M2N

        M2N -| p53

        DNA ->? DNA
        DNA -| M2N
    ").unwrap();*/

    let model = BooleanNetwork::try_from(
        "
        SSF -> SWI5

        SSF -> ACE2

        SBF -> SSF
        HCM1 -> SSF

        MBF -> YHP1
        SBF -> YHP1

        MBF -> HCM1
        SBF -> HCM1

        MBF -> YOX1
        SBF -> YOX1

        CLN3 -> SBF
        MBF -> SBF
        YHP1 -| SBF
        YOX1 -| SBF

        CLN3 -> MBF

        ACE2 -> CLN3
        YHP1 -| CLN3
        SWI5 -> CLN3
        YOX1 -| CLN3
# Problematic model - according to the new implementation, it oscillates
#$ACE2: SSF
#$CLN3: (ACE2 | (SWI5 & (!YHP1 | !YOX1)))
#$HCM1: (MBF & SBF)
#$MBF: CLN3
#$SBF: ((CLN3 & (MBF | (!YHP1 | !YOX1))) | (!CLN3 & (MBF | (!YHP1 & !YOX1))))
#$SSF: (HCM1 & SBF)
#$SWI5: SSF
#$YHP1: (MBF | SBF)
#$YOX1: (MBF | SBF)
    ",
    )
    .unwrap();
    /*let model = BooleanNetwork::try_from(
        "\
        start -> SK

        Cdc2 -| Ste9
        PP -> Ste9
        SK -| Ste9
        Ste9 -> Ste9
        Cdc2A -| Ste9

        Cdc2 -| Rum1
        PP -> Rum1
        SK -| Rum1
        Rum1 -> Rum1
        Cdc2A -| Rum1

        Ste9 -| Cdc2
        Rum1 -| Cdc2
        Slp1 -| Cdc2

        Cdc2 -> Cdc25
        Cdc25 -> Cdc25
        PP -| Cdc25

        Slp1 -> PP

        Cdc2A -> Slp1

        Cdc2 -| Wee1
        PP -> Wee1
        Wee1 -> Wee1

        Cdc25 -> Cdc2A
        Wee1 -| Cdc2A
        Ste9 -| Cdc2A
        Rum1 -| Cdc2A
        Slp1 -| Cdc2A
    ",
    )
    .unwrap();*/

    println!("Model: {}", model);

    for id in model.graph().variable_ids() {
        println!("Var: {}", model.graph().get_variable(id));
    }

    let graph = AsyncGraph::new(model).unwrap();

    println!("Unit BDD: {}", graph.unit_params().cardinality());

    let mut classifier = Classifier::new(&graph);
    components(&graph, |component| {
        let size = component.iter().count();
        println!("Component {}", size);
        /*println!("{:?}", component.cardinalities());
        let fwd = (&graph).fwd();
        for (s, _) in component.iter() {
            println!("Succ of {:?} are {:?}", s, fwd.step(s).filter_map(|(s, p)| {
                if p.is_empty() { None } else { Some(s) }
            }).collect::<Vec<_>>())
        }*/
        classifier.add_component(component);
    });

    classifier.print();
}
