use crate::scc::Behaviour;
use std::convert::TryFrom;

impl TryFrom<&str> for Behaviour {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "S" => Ok(Behaviour::Stability),
            "D" => Ok(Behaviour::Disorder),
            "O" => Ok(Behaviour::Oscillation),
            _ => Err(format!("Invalid behaviour string `{}`.", value)),
        }
    }
}

impl TryFrom<char> for Behaviour {
    type Error = String;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'D' => Ok(Behaviour::Disorder),
            'O' => Ok(Behaviour::Oscillation),
            'S' => Ok(Behaviour::Stability),
            _ => Err(format!("Invalid behavior: `{}`", value)),
        }
    }
}
