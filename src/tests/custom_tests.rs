//! Tests for custom validation

use crate::prelude::*;

#[test]
fn test_custom_check() {
    #[derive(RodValidate)]
    struct CustomField {
        #[rod(String)]
        field: String,
    }
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            CustomField,
            check = |x| {
                x.field.len() > 5
            }
        )]
        field: CustomField,
    }
    let test = Test {
        field: CustomField {
            field: "123456".to_string(),
        },
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: CustomField {
            field: "12345".to_string(),
        },
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}

#[test]
fn test_custom_check_complicated() {
    #[derive(RodValidate)]
    struct MyEntity {
        #[rod(
            String {
                length: 5..=10,
            },
            check = |s| {
                s.chars().all(|c| c.is_alphanumeric())
            }
        )]
        my_string: String,
    }
    let entity = MyEntity {
        my_string: "Hello123".to_string(),
    };
    assert!(entity.validate().is_ok());
}
