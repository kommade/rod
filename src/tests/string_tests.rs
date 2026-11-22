//! Tests for string validation

use crate::prelude::*;

#[test]
fn test_string_length() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            length: 5,
        })]
        field: String,
    }

    let test = Test {
        field: "12345".to_string(),
    };
    assert!(test.validate().is_ok());

    let test = Test {
        field: "1234".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_literals() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(Literal {
            is: "12345",
        })]
        field: String,
    }

    let test = Test {
        field: "12345".to_string(),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: "1234".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_length_enum() {
    #[derive(RodValidate)]
    enum TestEnum {
        First,
        Second,
        Third(
            #[rod(Literal {
                is: true,
            })]
            bool,
        )
    }
    
    let test = TestEnum::Third(true);
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = TestEnum::Third(false);
    assert!(test.validate().is_err());
}
