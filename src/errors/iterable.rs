use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum IterableValidation {
    Length(&'static str, usize, String),
}

impl Display for IterableValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IterableValidation::Length(path, actual_length, expected_length) => {
                write!(f, "Expected iterable at {} to have length {}, got {}", path, expected_length, actual_length)
            }
        }
    }
}