//! Tests for validation combinators

use crate::prelude::*;
use crate::combinators;

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
