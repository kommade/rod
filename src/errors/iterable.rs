use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum IterableValidation {
    Length(&'static str, usize, String),
    /// `distinct = true` was requested but the iterable contains duplicates.
    Distinct(&'static str),
}

impl Display for IterableValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IterableValidation::Length(path, actual_length, expected_length) => {
                write!(f, "Expected iterable at {} to have length {}, got {}", path, expected_length, actual_length)
            }
            IterableValidation::Distinct(path) => {
                write!(f, "Expected iterable at `{}` to have distinct items, found duplicates", path)
            }
        }
    }
}