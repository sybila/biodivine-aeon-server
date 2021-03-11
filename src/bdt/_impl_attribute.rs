use crate::bdt::{Attribute, BifurcationFunction};
use crate::util::functional::Functional;
use biodivine_lib_param_bn::biodivine_std::traits::Set;
use biodivine_lib_param_bn::symbolic_async_graph::GraphColors;
use std::collections::HashMap;

impl Attribute {
    /// Apply this attribute to the given bifurcation function, splitting it into two.
    pub fn split_function(
        &self,
        classes: &BifurcationFunction,
    ) -> (BifurcationFunction, BifurcationFunction) {
        (
            Self::restrict(classes, &self.negative),
            Self::restrict(classes, &self.positive),
        )
    }

    /// Restrict given bifurcation function using the specified attribute parameter set.
    fn restrict(classes: &BifurcationFunction, attribute: &GraphColors) -> BifurcationFunction {
        classes
            .iter()
            .filter_map(|(c, p)| {
                (c.clone(), attribute.intersect(p)).take_if(|(_, c)| !c.is_empty())
            })
            .collect::<HashMap<_, _>>()
    }

    /// Returns true when this attribute is a more specific version of the argument attribute.
    pub fn is_specification_of(&self, attr: &Attribute) -> bool {
        match (&self.context, &attr.context) {
            (Some(my_ctx), Some(their_ctx)) => {
                if my_ctx.regulator == their_ctx.regulator && my_ctx.target == their_ctx.target {
                    if my_ctx.context.len() > their_ctx.context.len() {
                        for v in &their_ctx.context {
                            if my_ctx.context.iter().find(|it| *it == v).is_none() {
                                return false;
                            }
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
