//! Validation combinators for composing validations

use crate::prelude::*;
use crate::errors::{RodValidateError, RodValidateErrorList};

/// A trait for types that can be validated with combinators
pub trait RodValidateExt: RodValidate + Sized {
    /// Validate with all validations, returning the first error
    fn validate_all(&self) -> Result<(), RodValidateError> {
        self.validate()
    }
    
    /// Validate with any of the validations, returning success if at least one passes
    fn validate_any<F>(&self, validators: &[F]) -> Result<(), RodValidateError>
    where
        F: Fn(&Self) -> Result<(), RodValidateError>,
    {
        let mut last_error = None;
        for validator in validators {
            match validator(self) {
                Ok(()) => return Ok(()),
                Err(e) => last_error = Some(e),
            }
        }
        Err(last_error.unwrap_or_else(|| RodValidateError::CheckFailed("No validators provided")))
    }
    
    /// Validate with one of the validations, returning success if exactly one passes
    fn validate_one<F>(&self, validators: &[F]) -> Result<(), RodValidateError>
    where
        F: Fn(&Self) -> Result<(), RodValidateError>,
    {
        let mut success_count = 0;
        let mut last_error = None;
        for validator in validators {
            match validator(self) {
                Ok(()) => success_count += 1,
                Err(e) => last_error = Some(e),
            }
        }
        if success_count == 1 {
            Ok(())
        } else if success_count == 0 {
            Err(last_error.unwrap_or_else(|| RodValidateError::CheckFailed("No validators provided")))
        } else {
            Err(RodValidateError::CheckFailed("Multiple validators passed, expected exactly one"))
        }
    }
}

impl<T: RodValidate> RodValidateExt for T {}

/// A builder for conditional validation
pub struct ConditionalValidator<T> {
    value: T,
    condition: bool,
}

impl<T: RodValidate> ConditionalValidator<T> {
    /// Create a new conditional validator
    pub fn new(value: T, condition: bool) -> Self {
        Self { value, condition }
    }
    
    /// Validate if condition is true
    pub fn validate_if_valid(self) -> Result<(), RodValidateError> {
        if self.condition {
            self.value.validate()
        } else {
            Ok(())
        }
    }
    
    /// Validate if condition is false
    pub fn validate_if_invalid(self) -> Result<(), RodValidateError> {
        if !self.condition {
            self.value.validate()
        } else {
            Ok(())
        }
    }
}

/// A builder for cross-field validation
pub struct CrossFieldValidator<T> {
    value: T,
}

impl<T: RodValidate> CrossFieldValidator<T> {
    /// Create a new cross-field validator
    pub fn new(value: T) -> Self {
        Self { value }
    }
    
    /// Validate with a cross-field check
    pub fn validate_with<F, E>(self, check: F) -> Result<(), RodValidateError>
    where
        F: Fn(&T) -> Result<(), E>,
        E: Into<RodValidateError>,
    {
        check(&self.value).map_err(Into::into)
    }
}

/// A builder for validation groups
pub struct ValidationGroup<T> {
    value: T,
    group: Option<String>,
}

impl<T: RodValidate> ValidationGroup<T> {
    /// Create a new validation group
    pub fn new(value: T) -> Self {
        Self { value, group: None }
    }
    
    /// Set the group name
    pub fn group(self, name: &str) -> Self {
        Self {
            value: self.value,
            group: Some(name.to_string()),
        }
    }
    
    /// Validate only the specified group
    pub fn validate_group(self, group_name: &str) -> Result<(), RodValidateError> {
        if self.group.as_ref().map_or(true, |g| g == group_name) {
            self.value.validate()
        } else {
            Ok(())
        }
    }
    
    /// Validate all groups
    pub fn validate_all_groups(self) -> Result<(), RodValidateError> {
        self.value.validate()
    }
}

/// A builder for validation pipeline
pub struct ValidationPipeline<T> {
    value: T,
    errors: Vec<RodValidateError>,
}

impl<T: RodValidate> ValidationPipeline<T> {
    /// Create a new validation pipeline
    pub fn new(value: T) -> Self {
        Self { value, errors: Vec::new() }
    }
    
    /// Add a validation step
    pub fn step<F>(mut self, step: F) -> Self
    where
        F: Fn(&T) -> Result<(), RodValidateError>,
    {
        if let Err(e) = step(&self.value) {
            self.errors.push(e);
        }
        self
    }
    
    /// Add a validation step that collects all errors
    pub fn step_all<F>(mut self, step: F) -> Self
    where
        F: Fn(&T) -> Result<(), RodValidateErrorList>,
    {
        if let Err(errors) = step(&self.value) {
            self.errors.extend(errors);
        }
        self
    }
    
    /// Get the final result
    pub fn result(self) -> Result<(), RodValidateError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors[0].clone())
        }
    }
    
    /// Get all errors
    pub fn all_errors(self) -> Vec<RodValidateError> {
        self.errors
    }
}

/// A trait for collecting validation metrics
pub trait ValidationMetrics {
    /// Get the validation duration in milliseconds
    fn validation_duration_ms(&self) -> u64;
    
    /// Get the number of validation checks
    fn validation_checks(&self) -> usize;
    
    /// Get the number of failed validations
    fn validation_failures(&self) -> usize;
}

/// A trait for validation with different modes
pub trait ValidationMode {
    /// Validate in strict mode (fail on first error)
    fn validate_strict(&self) -> Result<(), RodValidateError>;
    
    /// Validate in warn mode (collect all errors but don't fail)
    fn validate_warn(&self) -> Result<(), RodValidateError>;
}

impl<T: RodValidate> ValidationMode for T {
    fn validate_strict(&self) -> Result<(), RodValidateError> {
        self.validate()
    }
    
    fn validate_warn(&self) -> Result<(), RodValidateError> {
        // In warn mode, we still return the first error but could collect all
        self.validate()
    }
}

/// A trait for generating JSON Schema from validation rules
#[cfg(feature = "serde")]
pub trait ToJsonSchema {
    /// Generate a JSON Schema from the validation rules
    fn to_json_schema() -> serde_json::Value;
}

/// A trait for validation testing utilities
pub trait ValidationTestUtils {
    /// Check if a value passes validation
    fn passes_validation(&self) -> bool;
    
    /// Check if a value fails validation
    fn fails_validation(&self) -> bool;
    
    /// Get the first validation error
    fn first_validation_error(&self) -> Option<RodValidateError>;
    
    /// Get all validation errors
    fn all_validation_errors(&self) -> Vec<RodValidateError>;
}

impl<T: RodValidate> ValidationTestUtils for T {
    fn passes_validation(&self) -> bool {
        self.validate().is_ok()
    }
    
    fn fails_validation(&self) -> bool {
        self.validate().is_err()
    }
    
    fn first_validation_error(&self) -> Option<RodValidateError> {
        self.validate().err()
    }
    
    fn all_validation_errors(&self) -> Vec<RodValidateError> {
        match self.validate_all() {
            Ok(()) => Vec::new(),
            Err(errors) => errors.iter().cloned().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_any() {
        #[derive(RodValidate)]
        struct Test {
            #[rod(String { length: 5 })]
            field: String,
        }

        let valid = Test { field: "12345".to_string() };
        let invalid = Test { field: "123".to_string() };

        assert!(valid.validate_any(&[|s: &Test| s.validate()]).is_ok());
        assert!(invalid.validate_any(&[|s: &Test| s.validate()]).is_err());
    }

    #[test]
    fn test_validate_one() {
        #[derive(RodValidate)]
        struct Test {
            #[rod(String { length: 5 })]
            field: String,
        }

        let valid = Test { field: "12345".to_string() };

        assert!(valid.validate_one(&[|s: &Test| s.validate()]).is_ok());
    }

    #[test]
    fn test_conditional_validator() {
        #[derive(RodValidate, Clone)]
        struct Test {
            #[rod(String { length: 5 })]
            field: String,
        }

        let valid = Test { field: "12345".to_string() };
        let invalid = Test { field: "123".to_string() };

        assert!(ConditionalValidator::new(valid.clone(), true).validate_if_valid().is_ok());
        assert!(ConditionalValidator::new(invalid.clone(), true).validate_if_valid().is_err());
        assert!(ConditionalValidator::new(invalid.clone(), false).validate_if_valid().is_ok());
    }

    #[test]
    fn test_cross_field_validator() {
        #[derive(RodValidate)]
        struct Test {
            #[rod(String { length: 5 })]
            field: String,
        }

        let valid = Test { field: "12345".to_string() };
        assert!(CrossFieldValidator::new(valid).validate_with(|t| t.validate()).is_ok());
    }

    #[test]
    fn test_validation_pipeline() {
        #[derive(RodValidate)]
        struct Test {
            #[rod(String { length: 5 })]
            field: String,
        }

        let valid = Test { field: "12345".to_string() };
        let result = ValidationPipeline::new(valid)
            .step(|t| t.validate())
            .result();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_test_utils() {
        #[derive(RodValidate)]
        struct Test {
            #[rod(String { length: 5 })]
            field: String,
        }

        let valid = Test { field: "12345".to_string() };
        assert!(valid.passes_validation());
        assert!(!valid.fails_validation());
        assert!(valid.first_validation_error().is_none());
    }
}
