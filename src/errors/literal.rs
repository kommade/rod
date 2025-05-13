use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum LiteralValidation {
    Value(String, String),
}

impl Display for LiteralValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValidation::Value(value, expected) => write!(f, "Expected value {}, got {}", expected, value),
        }
    }
}