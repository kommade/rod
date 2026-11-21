//! Tests for iterable validation

use crate::prelude::*;

#[test]
fn test_iterable() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Iterable {
                item: i32 {
                    size: 6..=8,
                    sign: Positive,
                    step: 2,
                },
                length: 2,
            }
        )]
        field: Vec<i32>,
    }
    let test = Test {
        field: vec![6, 8],
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: vec![5, 7],
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: vec![6, 8, 10],
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: vec![6],
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}
