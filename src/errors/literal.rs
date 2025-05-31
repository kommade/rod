use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum LiteralValidation {
    Value(&'static str, String, String),
}

impl Display for LiteralValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValidation::Value(path, value, expected) => write!(f, "Expected `{}` to be {}, got {}", path, expected, value),
        }
    }
}