use crate::all::{
    ALLAtom, ALLFormula, AttractorAtom, AttractorFormula, BooleanFormula, StateAtom, StateFormula,
};
use crate::scc::Behaviour;
use biodivine_lib_param_bn::{BinaryOp, BooleanNetwork};
use regex::internal::Input;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Clone, Eq, PartialEq, Debug)]
enum BooleanExpressionToken<A>
where
    A: Eq,
{
    Not,
    BinaryOp(BinaryOp),
    Tokens(Vec<BooleanExpressionToken<A>>),
    Atom(A),
}

type StateFormulaToken = BooleanExpressionToken<String>;

fn tokenize_boolean_expression<A, F>(
    data: &mut Peekable<Chars>,
    top_level: bool,
    try_tokenize_atom: &F,
) -> Result<Vec<BooleanExpressionToken<A>>, String>
where
    F: Fn(&mut Peekable<Chars>, char) -> Option<A>,
    A: Eq,
{
    let mut output = Vec::new();
    while let Some(c) = data.next() {
        match c {
            c if c.is_whitespace() => { /* Skip whitespace */ }
            // single char tokens
            '!' => output.push(BooleanExpressionToken::Not),
            '&' => output.push(BooleanExpressionToken::BinaryOp(BinaryOp::And)),
            '|' => output.push(BooleanExpressionToken::BinaryOp(BinaryOp::Or)),
            '^' => output.push(BooleanExpressionToken::BinaryOp(BinaryOp::Xor)),
            '=' => {
                if Some('>') == data.next() {
                    output.push(BooleanExpressionToken::BinaryOp(BinaryOp::Imp));
                } else {
                    return Result::Err("Expected '>' after '='.".to_string());
                }
            }
            '<' => {
                if Some('=') == data.next() {
                    if Some('>') == data.next() {
                        output.push(BooleanExpressionToken::BinaryOp(BinaryOp::Iff))
                    } else {
                        return Result::Err("Expected '>' after '='.".to_string());
                    }
                } else {
                    return Result::Err("Expected '=' after '<'.".to_string());
                }
            }
            // '>' is invalid as a start of a token
            '>' => return Result::Err("Unexpected '>'.".to_string()),
            ')' => {
                return if !top_level {
                    Result::Ok(output)
                } else {
                    Result::Err("Unexpected ')'.".to_string())
                }
            }
            '(' => {
                // start a nested token group
                let tokens = tokenize_boolean_expression(data, false, try_tokenize_atom)?;
                output.push(BooleanExpressionToken::Tokens(tokens));
            }
            c => {
                if let Some(atom) = try_tokenize_atom(data, c) {
                    output.push(BooleanExpressionToken::Atom(atom));
                } else {
                    return Result::Err(format!("Unexpected '{}'.", c));
                }
            }
        }
    }
    return if top_level {
        Result::Ok(output)
    } else {
        Result::Err("Expected ')'.".to_string())
    };
}

fn try_string_atom(data: &mut Peekable<Chars>, first: char) -> Option<String> {
    return if is_valid_in_name(first) {
        // start of a variable name
        let mut name = vec![first];
        while let Some(c) = data.peek() {
            if c.is_whitespace() || !is_valid_in_name(*c) {
                break;
            } else {
                name.push(*c);
                data.next(); // advance iterator
            }
        }
        Some(name.into_iter().collect())
    } else {
        None
    };
}

fn is_valid_in_name(c: char) -> bool {
    return c.is_alphanumeric() || c == '_' || c == '{' || c == '}';
}

fn parse_boolean_expression<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    return iff(data, parse_atom);
}

/// **(internal)** Utility method to find first occurrence of a specific token in the token tree.
fn index_of_first<A>(
    data: &[BooleanExpressionToken<A>],
    token: BooleanExpressionToken<A>,
) -> Option<usize>
where
    A: Eq,
{
    return data.iter().position(|t| *t == token);
}

/// **(internal)** Recursive parsing step 1: extract `<=>` operators.
fn iff<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    let iff_token = index_of_first(data, BooleanExpressionToken::BinaryOp(BinaryOp::Iff));
    return Ok(if let Some(i) = iff_token {
        Box::new(BooleanFormula::Binary {
            op: BinaryOp::Iff,
            left: imp(&data[..i], parse_atom)?,
            right: iff(&data[(i + 1)..], parse_atom)?,
        })
    } else {
        imp(data, parse_atom)?
    });
}

/// **(internal)** Recursive parsing step 2: extract `=>` operators.
fn imp<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    let imp_token = index_of_first(data, BooleanExpressionToken::BinaryOp(BinaryOp::Imp));
    return Ok(if let Some(i) = imp_token {
        Box::new(BooleanFormula::Binary {
            op: BinaryOp::Imp,
            left: imp(&data[..i], parse_atom)?,
            right: iff(&data[(i + 1)..], parse_atom)?,
        })
    } else {
        or(data, parse_atom)?
    });
}

/// **(internal)** Recursive parsing step 3: extract `|` operators.
fn or<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    let or_token = index_of_first(data, BooleanExpressionToken::BinaryOp(BinaryOp::Or));
    return Ok(if let Some(i) = or_token {
        Box::new(BooleanFormula::Binary {
            op: BinaryOp::Or,
            left: imp(&data[..i], parse_atom)?,
            right: iff(&data[(i + 1)..], parse_atom)?,
        })
    } else {
        and(data, parse_atom)?
    });
}

/// **(internal)** Recursive parsing step 4: extract `&` operators.
fn and<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    let and_token = index_of_first(data, BooleanExpressionToken::BinaryOp(BinaryOp::And));
    return Ok(if let Some(i) = and_token {
        Box::new(BooleanFormula::Binary {
            op: BinaryOp::And,
            left: imp(&data[..i], parse_atom)?,
            right: iff(&data[(i + 1)..], parse_atom)?,
        })
    } else {
        xor(data, parse_atom)?
    });
}

/// **(internal)** Recursive parsing step 5: extract `^` operators.
fn xor<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    let xor_token = index_of_first(data, BooleanExpressionToken::BinaryOp(BinaryOp::Xor));
    return Ok(if let Some(i) = xor_token {
        Box::new(BooleanFormula::Binary {
            op: BinaryOp::Xor,
            left: imp(&data[..i], parse_atom)?,
            right: iff(&data[(i + 1)..], parse_atom)?,
        })
    } else {
        terminal(data, parse_atom)?
    });
}

/// **(internal)** Recursive parsing step 6: extract terminals and negations.
fn terminal<AT, AF, F>(
    data: &[BooleanExpressionToken<AT>],
    parse_atom: &F,
) -> Result<Box<BooleanFormula<AF>>, String>
where
    F: Fn(&AT) -> Result<AF, String>,
    AT: Eq,
{
    return if data.is_empty() {
        Err("Expected formula, found nothing.".to_string())
    } else if data[0] == BooleanExpressionToken::Not {
        Ok(Box::new(BooleanFormula::Not(terminal(
            &data[1..],
            parse_atom,
        )?)))
    } else if data.len() != 1 {
        Err("Too many atom tokens in a formula.".to_string())
    } else {
        match &data[0] {
            BooleanExpressionToken::Atom(atom) => {
                Ok(Box::new(BooleanFormula::Atom(parse_atom(atom)?)))
            }
            BooleanExpressionToken::Tokens(inner) => {
                Ok(parse_boolean_expression(inner, parse_atom)?)
            }
            _ => Err("Expected atom token.".to_string()),
        }
    };
}

pub fn parse_filter(bn: &BooleanNetwork, value: &str) -> Result<ALLFormula, String> {
    let tokens: Vec<BooleanExpressionToken<ALLAtom>> =
        tokenize_boolean_expression(&mut value.chars().peekable(), true, &|data, first| {
            if is_valid_in_name(first) {
                // start of a variable name
                let mut name = vec![first];
                while let Some(c) = data.peek() {
                    if c.is_whitespace() || !is_valid_in_name(*c) {
                        break;
                    } else {
                        name.push(*c);
                        data.next(); // advance iterator
                    }
                }
                let name: String = name.into_iter().collect();
                println!("Tokenize (ALL) {}", name);
                if name == "AllAttractors" || name == "SomeAttractor" {
                    while let Some(c) = data.peek() {
                        if c.is_whitespace() {
                            data.next();
                        } else if *c == '(' {
                            data.next();
                            // TRY TO TOKENIZE ATTRACTOR ATOM
                            if let Ok(tokens) =
                                tokenize_boolean_expression(data, false, &|data, first| {
                                    return if is_valid_in_name(first) {
                                        // start of a variable name
                                        let mut name = vec![first];
                                        while let Some(c) = data.peek() {
                                            if c.is_whitespace() || !is_valid_in_name(*c) {
                                                break;
                                            } else {
                                                name.push(*c);
                                                data.next(); // advance iterator
                                            }
                                        }
                                        let name: String = name.into_iter().collect();
                                        println!("Tokenize (Attractor) {}", name);
                                        if name == "AllStates" || name == "SomeState" {
                                            while let Some(c) = data.peek() {
                                                if c.is_whitespace() {
                                                    data.next();
                                                } else if *c == '(' {
                                                    data.next();
                                                    // TRY TO TOKENIZE STRING ATOM (STATE FORMULA)
                                                    if let Ok(tokens) = tokenize_boolean_expression(
                                                        data,
                                                        false,
                                                        &self::try_string_atom,
                                                    ) {
                                                        if let Ok(state_formula) =
                                                            parse_boolean_expression(
                                                                &tokens,
                                                                &|atom: &String| {
                                                                    Ok(StateAtom::IsSet(
                                                                        bn.graph()
                                                                            .find_variable(
                                                                                atom.as_str(),
                                                                            )
                                                                            .unwrap(),
                                                                    ))
                                                                },
                                                            )
                                                        {
                                                            return if name == "AllStates" {
                                                                Some(AttractorAtom::AllStates(
                                                                    *state_formula,
                                                                ))
                                                            } else {
                                                                Some(AttractorAtom::SomeState(
                                                                    *state_formula,
                                                                ))
                                                            };
                                                        }
                                                    } else {
                                                        return None;
                                                    }
                                                }
                                            }
                                        } else if name == "stability" {
                                            return Some(AttractorAtom::IsClass(
                                                Behaviour::Stability,
                                            ));
                                        } else if name == "oscillation" {
                                            return Some(AttractorAtom::IsClass(
                                                Behaviour::Oscillation,
                                            ));
                                        } else if name == "disorder" {
                                            return Some(AttractorAtom::IsClass(
                                                Behaviour::Disorder,
                                            ));
                                        }
                                        None
                                    } else {
                                        None
                                    };
                                })
                            {
                                return if let Ok(attractor_formula) =
                                    parse_boolean_expression(&tokens, &|atom| Ok(atom.clone()))
                                {
                                    if name == "AllAttractors" {
                                        Some(ALLAtom::AllAttractors(*attractor_formula))
                                    } else {
                                        Some(ALLAtom::SomeAttractor(*attractor_formula))
                                    }
                                } else {
                                    None
                                };
                            } else {
                                return None;
                            }
                        }
                    }
                }
                return None;
            } else {
                return None;
            }
        })?;
    let attractor_formula = parse_boolean_expression(&tokens, &|atom: &ALLAtom| Ok(atom.clone()))?;
    return Ok(*attractor_formula);
}

#[cfg(test)]
mod tests {
    use crate::all::parser::parse_filter;
    use biodivine_lib_param_bn::BooleanNetwork;
    use std::convert::TryFrom;

    #[test]
    fn tokenize_state_formula() {
        let bn = BooleanNetwork::try_from(
            "\
            a -> b
            a -> c
        ",
        )
        .unwrap();
        let input =
            "SomeAttractor(AllStates(a & b | c) & SomeState(c <=> b)) && AllAttractors(disorder)";
        println!("Parsed: {:?}", parse_filter(&bn, input))
    }
}
