use std::collections::HashMap;
use biodivine_lib_param_bn::BooleanNetwork;
use biodivine_aeon_server::control::temporary_source_target_control;
use std::convert::TryFrom;


#[allow(non_snake_case)]
fn main() {

    // This is the G2A network which is often used as an example, but here, we have added
    // an a self-loop to all variables that otherwise don't have one (because in a controlled
    // graph, technically every variable has a self-loop).

    // We have also given each variable an explicit update function.
    let g2a_network = r"
            $CtrA:f_5(CtrA, GcrA, CcrM, SciP)
            CtrA -> CtrA
            GcrA -> CtrA
            CcrM -| CtrA
            SciP -| CtrA
            $GcrA:f_4(CtrA, DnaA)
            GcrA -?? GcrA
            CtrA -| GcrA
            DnaA -> GcrA
            $CcrM:f_3(CtrA, CcrM, SciP)
            CtrA -> CcrM
            CcrM -| CcrM
            SciP -| CcrM
            $SciP:f_1(CtrA, DnaA)
            CtrA -> SciP
            DnaA -| SciP
            SciP -?? SciP
            $DnaA:f_2(CtrA, GcrA, DnaA, CcrM)
            CtrA -> DnaA
            GcrA -| DnaA
            DnaA -| DnaA
            CcrM -> DnaA
        ";

    let network = BooleanNetwork::try_from(g2a_network).unwrap();
    let CcrM = network.as_graph().find_variable("CcrM").unwrap();
    let CtrA = network.as_graph().find_variable("CtrA").unwrap();
    let DnaA = network.as_graph().find_variable("DnaA").unwrap();
    let GcrA = network.as_graph().find_variable("GcrA").unwrap();
    let SciP = network.as_graph().find_variable("SciP").unwrap();

    let mut source = HashMap::new();
    source.insert(CcrM, true);
    source.insert(CtrA, true);
    source.insert(DnaA, true);
    source.insert(GcrA, false);
    source.insert(SciP, false);

    let mut target = HashMap::new();
    target.insert(CcrM, true);
    target.insert(CtrA, false);
    target.insert(DnaA, false);
    target.insert(GcrA, false);
    target.insert(SciP, false);

    temporary_source_target_control(network, &source, &target);
}