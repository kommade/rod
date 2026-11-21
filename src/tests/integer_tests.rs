//! Tests for integer validation

use crate::prelude::*;

#[test]
fn test_integer() {
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
    }
    let test = Test {
        field: 6,
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: 5,
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}
