//! Tests for tuple validation

use crate::prelude::*;

#[test]
fn test_tuple() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Tuple (
                i32 {
                    size: 6..8,
                    sign: Positive,
                    step: 2,
                },
                i32 {
                    size: 6..=8,
                    sign: Positive,
                    step: 2,
                }
            )
        )]
        field: (i32, i32),
    }
    let test = Test {
        field: (6, 8),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: (5, 7),
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}

#[test]
fn test_tuple_nested() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Tuple (
                i32 {
                    size: 6..8,
                    sign: Positive,
                    step: 2,
                },
                Tuple (
                    i32 {
                        size: 6..=8,
                        sign: Positive,
                        step: 2,
                    },
                    i32 {
                        size: 6..=8,
                        sign: Positive,
                        step: 2,
                    }
                )
            )
        )]
        field: (i32, (i32, i32)),
    }
    let test = Test {
        field: (6, (6, 8)),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: (5, (6, 8)),
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}

#[test]
fn test_tuple_struct() {
    #[derive(RodValidate)]
    struct InsideTuple {
        #[rod(
            i32 {
                size: 6..8,
                sign: Positive,
                step: 2,
            }
        )]
        field: i32,
    }
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Tuple (
                InsideTuple,
                i32 {
                    size: 6..=8,
                    sign: Positive,
                    step: 2,
                }
            )
        )]
        field: (InsideTuple, i32),
        #[rod(skip)]
        other_field: i32,
    }
    let test = Test {
        field: (InsideTuple { field: 6 }, 8),
        other_field: 10,
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
    let test = Test {
        field: (InsideTuple { field: 5 }, 8),
        other_field: 10,
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}
