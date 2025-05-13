use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum OptionValidation {
    // Is None when the value should be Some
    None(&'static str),
    // Is Some when the value should be None
    Some(String),
}

impl Display for OptionValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionValidation::None(n) => write!(f, "Expected option to be {}, got None", n),
            OptionValidation::Some(s) => write!(f, "Expected option to be None, got {}", s),
        }
    }
}