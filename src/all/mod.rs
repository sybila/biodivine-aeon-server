use crate::scc::Behaviour;
use biodivine_lib_param_bn::{BinaryOp, VariableId};

pub mod parser;

// TODO: Something like this should go into standard library
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BooleanFormula<P> {
    Binary {
        op: BinaryOp,
        left: Box<BooleanFormula<P>>,
        right: Box<BooleanFormula<P>>,
    },
    Not(Box<BooleanFormula<P>>),
    Atom(P),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateAtom {
    IsSet(VariableId),
    IsNotSet(VariableId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttractorAtom {
    IsClass(Behaviour),
    AllStates(StateFormula),
    SomeState(StateFormula),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AllAtom {
    AllAttractors(AttractorFormula),
    SomeAttractor(AttractorFormula),
}

pub type StateFormula = BooleanFormula<StateAtom>;

pub type AttractorFormula = BooleanFormula<AttractorAtom>;

pub type AllFormula = BooleanFormula<AllAtom>;
/*
impl ALLFormula {
    pub fn eval(
        &self,
        attractors: &Vec<(StateSet, HashMap<Behaviour, BddParams>)>,
        graph: &SymbolicAsyncGraph,
    ) -> BddParams {
        return match self {
            BooleanFormula::Not(inner) => graph.unit_params().minus(&inner.eval(attractors, graph)),
            BooleanFormula::Binary { op, left, right } => {
                let left = left.eval(attractors, graph).into_bdd();
                let right = right.eval(attractors, graph).into_bdd();
                BddParams::from(match op {
                    BinaryOp::And => left.and(&right),
                    BinaryOp::Or => left.or(&right),
                    BinaryOp::Iff => left.iff(&right),
                    BinaryOp::Imp => left.imp(&right),
                    BinaryOp::Xor => left.xor(&right),
                })
            }
            BooleanFormula::Atom(ALLAtom::AllAttractors(af)) => {
                let mut result = graph.unit_params().clone();
                for (attractor, classification) in attractors {
                    if result.is_empty() {
                        return result;
                    } // end early
                    result = result.intersect(&af.eval(attractor, graph, classification));
                }
                result
            }
            BooleanFormula::Atom(ALLAtom::SomeAttractor(af)) => {
                let mut result = graph.empty_params();
                for (attractor, classification) in attractors {
                    result = result.union(&af.eval(attractor, graph, classification));
                }
                result
            }
        };
    }
}

impl StateFormula {
    pub fn eval(&self, state: IdState) -> bool {
        return match self {
            BooleanFormula::Atom(StateAtom::IsSet(id)) => state.get_bit((*id).into()),
            BooleanFormula::Atom(StateAtom::IsNotSet(id)) => !state.get_bit((*id).into()),
            BooleanFormula::Not(inner) => !inner.eval(state),
            BooleanFormula::Binary { op, left, right } => {
                let left = left.eval(state);
                let right = right.eval(state);
                match op {
                    BinaryOp::And => left && right,
                    BinaryOp::Or => left || right,
                    BinaryOp::Iff => left == right,
                    BinaryOp::Imp => !left || right,
                    BinaryOp::Xor => left != right,
                }
            }
        };
    }
}

impl AttractorFormula {
    pub fn eval(
        &self,
        attractor: &StateSet,
        graph: &SymbolicAsyncGraph,
        classification: &HashMap<Behaviour, BddParams>,
    ) -> BddParams {
        return match self {
            BooleanFormula::Binary { op, left, right } => {
                let left = left.eval(attractor, graph, classification).into_bdd();
                let right = right.eval(attractor, graph, classification).into_bdd();
                BddParams::from(match op {
                    BinaryOp::And => left.and(&right),
                    BinaryOp::Or => left.or(&right),
                    BinaryOp::Iff => left.iff(&right),
                    BinaryOp::Imp => left.imp(&right),
                    BinaryOp::Xor => left.xor(&right),
                })
            }
            BooleanFormula::Not(inner) => {
                return attractor
                    .fold_union()
                    .unwrap_or(graph.empty_params())
                    .minus(&inner.eval(attractor, graph, classification))
            }
            BooleanFormula::Atom(AttractorAtom::AllStates(sf)) => {
                let mut result = attractor.fold_union().unwrap_or(graph.empty_params());
                for (s, p) in attractor.iter() {
                    if result.is_empty() {
                        return result;
                    } // end early
                    if !sf.eval(s) {
                        result = result.minus(p)
                    }
                }
                result
            }
            BooleanFormula::Atom(AttractorAtom::SomeState(sf)) => {
                let mut result = graph.empty_params();
                for (s, p) in attractor.iter() {
                    if sf.eval(s) {
                        result = result.union(p)
                    }
                }
                result
            }
            BooleanFormula::Atom(AttractorAtom::IsClass(cls)) => classification
                .get(cls)
                .unwrap_or(&graph.empty_params())
                .clone(),
        };
    }
}
*/
