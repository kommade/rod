use std::fmt::{Display, Formatter};

use super::{Integer};

#[derive(Debug, Clone)]
pub enum IntegerValidation {
    Size(Integer, String),
    Sign(Integer, &'static str),
    Step(Integer, Integer),
}

impl Display for IntegerValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerValidation::Size(int, size) => write!(f, "Expected number {}, got {}", size, int),
            IntegerValidation::Sign(int, sign) => write!(f, "Expected number with sign {}, got {}", sign, int),
            IntegerValidation::Step(int, step) => write!(f, "Expected number with step: {}, got {}", step, int),
        }
    }
}