//! Tests for float validation

use crate::prelude::*;

#[test]
fn test_float() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            f64 {
                size: 2.0..=4.0,
                sign: Positive,
            }
        )]
        field: f64,
    }
    let test = Test {
        field: 3.0,
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: 5.0,
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}
