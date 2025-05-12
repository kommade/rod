#![allow(unused_imports)]
#![allow(dead_code)]

use rod_derive::RodValidate;
use crate::*;

#[test]
fn test_basic() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            String {
                length: 5..=10,
                format: "^[a-zA-Z]+$",
            }
        )]
        field1: String,
        #[rod(
            i32 {
                range: 0..=100,
            }
        )]
        field2: i32,
    }

    let test = Test {
        field1: String::from("testing"),
        field2: 42,
    };
    let result = test.validate();
    assert!(result.is_ok(), "Validation failed: {:?}", result);
}

#[test]
fn test_child() {
    #[derive(RodValidate)]
    struct ImplementsRodValidate {
        #[rod(none)]
        field: String,
    }

    #[derive(RodValidate)]
    struct Test {
        #[rod(
            String {
                length: 5..=10,
                format: "^[a-zA-Z]+$",
            }
        )]
        field1: String,
        // #[rod(
        //     i32 {
        //         range: 0..=100,
        //     }
        // )]
        field2: ImplementsRodValidate,
    }
}