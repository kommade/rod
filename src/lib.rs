#[allow(unused_imports)]

mod tests;

#[derive(Debug, Clone)]
pub enum RodValidateError {
    InvalidLength,
    InvalidFormat,
    OutOfRange,
}

pub trait RodValidate {
    fn validate(&self) -> Result<(), RodValidateError>;
}

pub use rod_derive::RodValidate;