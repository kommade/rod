use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum FloatValidation {
    Size(&'static str, f64, String),
    Sign(&'static str, f64, &'static str),
    /// The float failed its `ftype` check; the third field names the
    /// required dtype (`NaN`, `Finite`, `Infinite`, `Normal`, `Subnormal`).
    Type(&'static str, f64, &'static str),
    /// The float is not a multiple of the `step` value (third field, kept
    /// as the user wrote it in the attribute).
    Step(&'static str, f64, String),
}

impl Display for FloatValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FloatValidation::Size(path, float, size) => write!(f, "Expected `{}` to be a float {}, got {}", path, size, float),
            FloatValidation::Sign(path, float, sign) => write!(f, "Expected `{}` to be a float with sign {}, got {}", path, float, sign),
            FloatValidation::Type(path, float, dtype) => write!(f, "Expected `{}` to be a float of type {}, got {}", path, dtype, float),
            FloatValidation::Step(path, float, step) => write!(f, "Expected `{}` to be a multiple of {}, got {}", path, step, float),
        }
    }
}