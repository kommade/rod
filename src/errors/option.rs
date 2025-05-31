use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum OptionValidation {
    // Is None when the value should be Some
    None(&'static str, &'static str),
    // Is Some when the value should be None
    Some(&'static str, String),
}

impl Display for OptionValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionValidation::None(path, n) => write!(f, "Expected `{}` to be {}, got None", path, n),
            OptionValidation::Some(path, s) => write!(f, "Expected `{}` to be None, got {}", path, s),
        }
    }
}