use crate::scc::algo_stability_analysis::Stability;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

impl Display for Stability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stability::True => write!(f, "true"),
            Stability::False => write!(f, "false"),
            Stability::Unstable => write!(f, "unstable"),
        }
    }
}

impl TryFrom<&str> for Stability {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "true" => Ok(Stability::True),
            "false" => Ok(Stability::False),
            "unstable" => Ok(Stability::Unstable),
            _ => Err(format!("Invalid stability value `{}`.", value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scc::algo_stability_analysis::Stability;
    use std::convert::TryFrom;

    #[test]
    pub fn stability_serialisation() {
        assert_eq!(
            Stability::True,
            Stability::try_from(Stability::True.to_string().as_str()).unwrap()
        );
        assert_eq!(
            Stability::False,
            Stability::try_from(Stability::False.to_string().as_str()).unwrap()
        );
        assert_eq!(
            Stability::Unstable,
            Stability::try_from(Stability::Unstable.to_string().as_str()).unwrap()
        );
        assert!(Stability::try_from("TRUE").is_err());
    }
}
