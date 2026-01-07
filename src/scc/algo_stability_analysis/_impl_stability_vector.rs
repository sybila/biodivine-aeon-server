use crate::scc::algo_stability_analysis::{Stability, StabilityVector};
use crate::util::functional::Functional;
use json::JsonValue;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::ops::Shr;

impl Display for StabilityVector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        if self.has_true {
            if self.has_false || self.has_unstable {
                write!(f, "true,")?;
            } else {
                write!(f, "true")?;
            }
        }
        if self.has_false {
            if self.has_unstable {
                write!(f, "false,")?;
            } else {
                write!(f, "false")?;
            }
        }
        if self.has_unstable {
            write!(f, "unstable")?;
        }
        write!(f, "]")
    }
}

impl From<StabilityVector> for usize {
    fn from(vector: StabilityVector) -> Self {
        0usize.apply(|id| {
            if vector.has_true {
                *id |= 0b1;
            }
            if vector.has_false {
                *id |= 0b10;
            }
            if vector.has_unstable {
                *id |= 0b100;
            }
        })
    }
}

impl TryFrom<usize> for StabilityVector {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let mut vector = StabilityVector::default();
        if value.shr(3) != 0usize {
            return Err(format!("Invalid stability vector id: `{}`.", value));
        }
        vector.has_true = (value & 0b1) != 0;
        vector.has_false = (value & 0b10) != 0;
        vector.has_unstable = (value & 0b100) != 0;
        Ok(vector)
    }
}

impl TryFrom<&str> for StabilityVector {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with('[') && value.ends_with(']') {
            let value = &value[1..value.len() - 1];
            let mut vector = StabilityVector::default();
            for part in value.split(',') {
                match part {
                    "true" => {
                        if vector.has_true {
                            return Err("Duplicate `true` in a stability vector.".to_string());
                        }
                        vector.has_true = true
                    }
                    "false" => {
                        if vector.has_false {
                            return Err("Duplicate `false` in a stability vector.".to_string());
                        }
                        vector.has_false = true
                    }
                    "unstable" => {
                        if vector.has_unstable {
                            return Err("Duplicate `unstable` in a stability vector.".to_string());
                        }
                        vector.has_unstable = true
                    }
                    _ => {
                        if !part.is_empty() {
                            return Err(format!("Unexpected `{}` in a stability vector.", part));
                        }
                    }
                }
            }
            Ok(vector)
        } else {
            Err(format!("Invalid stability vector: `{}`.", value))
        }
    }
}

impl StabilityVector {
    /// Create a new stability vector which includes the given stability value.
    ///
    /// If the value is already present, the current vector is only copied.
    pub fn add(&self, stability: Stability) -> StabilityVector {
        (*self).apply(|out| match stability {
            Stability::True => out.has_true = true,
            Stability::False => out.has_false = true,
            Stability::Unstable => out.has_unstable = true,
        })
    }

    pub fn is_empty(&self) -> bool {
        !(self.has_unstable || self.has_false || self.has_true)
    }

    pub fn export_json(&self) -> JsonValue {
        JsonValue::new_array().apply(|array| {
            if self.has_true {
                array.push("true").unwrap();
            }
            if self.has_false {
                array.push("false").unwrap();
            }
            if self.has_unstable {
                array.push("unstable").unwrap();
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::scc::algo_stability_analysis::StabilityVector;
    use std::convert::TryFrom;

    #[test]
    fn stability_to_string() {
        for id in 0..8usize {
            let vector = StabilityVector::try_from(id).unwrap();
            assert_eq!(
                vector,
                StabilityVector::try_from(vector.to_string().as_str()).unwrap()
            );
        }
        assert!(StabilityVector::try_from("true").is_err());
        assert!(StabilityVector::try_from("[true,true]").is_err());
        assert!(StabilityVector::try_from("[true-false]").is_err());
        assert!(StabilityVector::try_from("[true,false,false]").is_err());
        assert!(StabilityVector::try_from("[unstable,unstable,true]").is_err());
    }
}
