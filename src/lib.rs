//#[cfg(test)]
mod tests;
mod errors;
pub mod prelude;

pub trait RodValidate {
    fn validate(&self) -> Result<(), errors::RodValidateError>;
}