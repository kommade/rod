//#[cfg(test)]
mod tests;
mod errors;
pub use errors::*;

pub trait RodValidate {
    fn validate(&self) -> Result<(), RodValidateError>;
}

pub use rod_derive::RodValidate;