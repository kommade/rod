//! Tests for enum validation

use crate::prelude::*;

#[test]
fn test_enum_tuple() {
    #[derive(RodValidate)]
    enum TestEnum {
        First,
        Second,
        Third(
            #[rod(Literal {
                value: true,
            })]
            bool,
            #[rod(Literal {
                value: 1,
            })]
            i32,
        )
    }
    let test = TestEnum::Third(true, 1);
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = TestEnum::Third(false, 0);
    assert!(test.validate().is_err());
}

#[test]
fn test_enum_embedded_struct() {
    #[derive(RodValidate)]
    struct TestStruct {
        #[rod(f32 {
            size: 5.0..=10.0,
            sign: Positive,
        })]
        field: f32,
    }
    #[derive(RodValidate)]
    enum TestEnum {
        First(TestStruct),
        Second(
            #[rod(Literal {
                value: true,
            })]
            bool,
            #[rod(f64 {
                size: 5.0..=10.0,
                sign: Positive,
            })]
            f64,
        ),
        Third {
            #[rod(Literal {
                value: "12345",
            })]
            field: &'static str,
        }
    }
    let test = TestEnum::Third {
        field: "12345",
    };
}

#[test]
fn test_enum_with_reference() {
    #[derive(RodValidate)]
    enum TestEnum {
        First,
        Second(
            #[rod(
                i32 {
                    size: 6..8,
                    sign: Positive,
                    step: 2,
                }
            )]
            i32,
            #[rod(
                str {
                    length: 5,
                }
            )]
            &'static str,
        ),
    }
    let test = TestEnum::Second(6, "12345");
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = TestEnum::Second(5, "1234");
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}
