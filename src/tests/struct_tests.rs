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
