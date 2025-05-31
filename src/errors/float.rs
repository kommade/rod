use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum FloatValidation {
    Size(&'static str, f64, String),
    Sign(&'static str, f64, &'static str),
}

impl Display for FloatValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FloatValidation::Size(path, float, size) => write!(f, "Expected `{}` to be a float {}, got {}", path, size, float),
            FloatValidation::Sign(path, float, sign) => write!(f, "Expected `{}` to be a float with sign {}, got {}", path, float, sign),
        }
    }
}