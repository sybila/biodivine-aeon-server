/*pub fn run() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let model = BooleanNetwork::try_from(buffer.as_str()).unwrap();
    println!("Model: {}", model);
    println!("Model vars: {}", model.graph().num_vars());

    let graph = AsyncGraph::new(model).unwrap();

    println!("Unit BDD: {}", graph.unit_params().cardinality());

    let classifier = Classifier::new(&graph);
    let progress = ProgressTracker::new(&graph);
    components(&graph, &progress, &AtomicBool::new(false), |component| {
        let size = component.iter().count();
        println!("Component {}", size);
        /*println!("{:?}", component.cardinalities());
        let fwd = (&graph).fwd();
        for (s, _) in component.iter() {
            println!("Succ of {:?} are {:?}", s, fwd.step(s).filter_map(|(s, p)| {
                if p.is_empty() { None } else { Some(s) }
            }).collect::<Vec<_>>())
        }*/
        classifier.add_component(component, &graph);
    });

    classifier.print();
}*/

/*
# Problematic model - according to the new implementation, it oscillates
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
#$ACE2: SSF
#$CLN3: (ACE2 | (SWI5 & (!YHP1 | !YOX1)))
#$HCM1: (MBF & SBF)
#$MBF: CLN3
#$SBF: ((CLN3 & (MBF | (!YHP1 | !YOX1))) | (!CLN3 & (MBF | (!YHP1 & !YOX1))))
#$SSF: (HCM1 & SBF)
#$SWI5: SSF
#$YHP1: (MBF | SBF)
#$YOX1: (MBF | SBF)
*/
