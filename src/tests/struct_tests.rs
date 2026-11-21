//! Tests for struct validation

use crate::prelude::*;

#[test]
fn test_struct_with_reference() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                size: 6..8,
                sign: Positive,
                step: 2,
            }
        )]
        field: i32,
        #[rod(
            str {
                length: 5,
            }
        )]
        other_field: &'static str,
    }
    let test = Test {
        field: 6,
        other_field: "12345",
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: 5,
        other_field: "1234",
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}

#[test]
fn test_struct_double_reference() {
    static TEXT: &str = "12345";
    static LEVEL_REF: &'static i32 = &6;

    #[derive(RodValidate)]
    struct Test {
        #[rod(
            str {
                length: 5,
            }
        )]
        text: &'static &'static str,
        #[rod(
            i32 {
                size: 6..=8,
                sign: Positive,
                step: 2,
            }
        )]
        level: &'static &'static i32,
    }

    let test = Test {
        text: &TEXT,
        level: &LEVEL_REF,
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    assert!(test.validate_all().is_ok());

    static BAD_TEXT: &str = "12";
    static BAD_LEVEL_REF: &'static i32 = &5; // outside 6..=8 and not a multiple of 2
    let test = Test {
        text: &BAD_TEXT,
        level: &BAD_LEVEL_REF,
    };
    let errors = test.validate_all().unwrap_err();
    assert_eq!(errors.len(), 3, "{}", errors); // text length + level size + level step
}

#[test]
fn test_struct_triple_reference_custom_check() {
    static TEXT: &&str = &&"12345";

    #[derive(RodValidate)]
    struct Test {
        #[rod(
            str {
                length: 3..=10,
            },
            check = |s| s.chars().all(|c| c.is_alphanumeric())
        )]
        text: &'static &'static &'static str,
    }

    // The closure receives the innermost `&str`, same as for `&str` fields.
    let test = Test { text: &TEXT };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    static BAD: &&str = &&"ab-cd"; // length ok, check rejects the dash
    let test = Test { text: &BAD };
    let err = test.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::CheckFailed(path) if path == "text"), "{}", err);
}
