use crate::bdt::{BifurcationFunction, Attribute};
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use crate::util::functional::Functional;
use biodivine_lib_std::param_graph::Params;
use std::collections::HashMap;

impl Attribute {

    /// Apply this attribute to the given bifurcation function, splitting it into two.
    pub fn split_function(&self, classes: &BifurcationFunction) -> (BifurcationFunction, BifurcationFunction) {
        (Self::restrict(classes, &self.negative), Self::restrict(classes, &self.positive))
    }

    /// Restrict given bifurcation function using the specified attribute parameter set.
    fn restrict(classes: &BifurcationFunction, attribute: &GraphColors) -> BifurcationFunction {
        classes.iter()
            .filter_map(|(c,p)| {
                (c.clone(), attribute.intersect(p)).take_if(|(_, c)| !c.is_empty())
            })
            .collect::<HashMap<_, _>>()
    }

}
