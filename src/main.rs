use scc::algo_components::components;
use biodivine_lib_param_bn::BooleanNetwork;
use biodivine_lib_param_bn::async_graph::AsyncGraph;
use std::convert::TryFrom;
use crate::scc::Classifier;

mod scc;

fn main() {

    /*let model = BooleanNetwork::try_from("
        p53 -| DNA
        p53 -> M2C
        p53 -| M2N

        M2C -> M2N

        M2N -| p53

        DNA ->? DNA
        DNA -| M2N
    ").unwrap();*/

    let model = BooleanNetwork::try_from("
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
    ").unwrap();

    for id in model.graph().variable_ids() {
        println!("Var: {}", model.graph().get_variable(id));
    }

    let graph = AsyncGraph::new(model).unwrap();

    println!("Unit BDD: {}", graph.unit_params().cardinality());

    let mut classifier = Classifier::new(&graph);
    components(&graph, |component| {
        let size = component.iter().count();
        println!("Component {}", size);
        classifier.add_component(component);
    });

    classifier.print();

}
