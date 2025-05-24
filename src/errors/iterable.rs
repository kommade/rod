use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum IterableValidation {
    Length(String, String),
}

impl Display for IterableValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IterableValidation::Length(iterable, expected_length) => {
                write!(f, "Expected iterable of length {}, got {}", expected_length, iterable.len())
            }
        }
    }
}