use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum FloatValidation {
    Size(f64, String),
    Sign(f64, &'static str),
}

impl Display for FloatValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FloatValidation::Size(float, size) => write!(f, "Expected float {}, got {}", size, float),
            FloatValidation::Sign(float, sign) => write!(f, "Expected float with sign {}, got {}", sign, float),
        }
    }
}