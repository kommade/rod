//! Tests for error handling

use crate::prelude::*;

#[test]
fn test_validate_all() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                size: 6..=8,
                sign: Positive,
                step: 2,
            }
        )]
        field1: i32,
        #[rod(
            String {
                length: 5,
            }
        )]
        field2: String,
    }
    let test = Test {
        field1: 6,
        field2: "12345".to_string(),
    };
    assert!(test.validate_all().is_ok(), "{}", test.validate_all().unwrap_err());
    let test = Test {
        field1: 5,
        field2: "123456".to_string(),
    };
    assert!(test.validate_all().is_err() && test.validate_all().unwrap_err().len() == 3, "{}", test.validate_all().unwrap_err());
}

#[test]
fn test_user_defined_error() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                ?"hi"
                size: 6..=8,
                sign: Positive,
                step: 2,
            },
            message: "Field must be an even number between 6 and 8"
        )]
        field: i32,
        #[rod(
            String {
                length: 5,
            },
            message: "Field must be exactly 5 characters long"
        )]
        field2: String,
    }
    let test = Test {
        field: 5,
        field2: "1234".to_string(),
    };
    let err = test.validate_all().unwrap_err();
    assert!(err.len() == 3, "{}", err);
    assert!(err.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "hi")));
    assert!(err.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "Field must be an even number between 6 and 8")));
    assert!(err.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "Field must be exactly 5 characters long")));
}

#[cfg(feature = "regex")]
#[test]
fn test_per_validation_custom_errors() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                ?"int size"
                size: 6..=8,
                ?"int sign"
                sign: Negative,
                ?"int step"
                step: 2,
            }
        )]
        int_field: i32,
        #[rod(
            f64 {
                ?"float size"
                size: 2.0..=4.0,
                ?"float sign"
                sign: Negative,
                ?"float type"
                ftype: Finite,
            }
        )]
        float_field: f64,
        #[rod(
            String {
                ?"len"
                length: 5,
                ?"format"
                format: Email,
                ?"starts"
                starts_with: "Hi",
                ?"ends"
                ends_with: "!",
                ?"includes"
                includes: "abc",
            }
        )]
        string_field: String,
        #[rod(
            Literal {
                ?"literal"
                value: true,
            }
        )]
        literal_field: bool,
        #[rod(
            Option {
                ?"option"
                String {
                    ?"nested string"
                    length: 3,
                }
            }
        )]
        option_field: Option<String>,
        #[rod(
            Iterable {
                ?"iter length"
                length: 2,
                ?"iter item"
                item: String {
                    ?"iter item length"
                    length: 3,
                }
            }
        )]
        iterable_field: Vec<String>,
    }

    let test = Test {
        int_field: 5,
        float_field: f64::NAN,
        string_field: "bye".to_string(),
        literal_field: false,
        option_field: None,
        iterable_field: vec!["xx".to_string()],
    };

    let errors = test.validate_all().unwrap_err();
    assert_eq!(errors.len(), 15, "{}", errors);
    
    for expected in [
        "int size",
        "int sign",
        "int step",
        "float size",
        "float sign",
        "float type",
        "len",
        "format",
        "starts",
        "ends",
        "includes",
        "literal",
        "option",
        "iter length",
        "iter item length",
    ] {
        assert!(errors.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == expected)), "Missing expected message `{}` in errors: {}", expected, errors);
    }
}
