#[cfg(test)]
mod tests;
mod combinators;
mod errors;
pub mod prelude;

pub use combinators::*;

pub trait RodValidate {
    /// Validate the struct, returning an error if validation fails.
    fn validate(&self) -> Result<(), errors::RodValidateError>;
    /// Validate the struct, returning a list of errors if validation fails.
    fn validate_all(&self) -> Result<(), errors::RodValidateErrorList>;
}