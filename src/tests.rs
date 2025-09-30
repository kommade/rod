#![allow(unused)]

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
            value: "12345",
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
                value: true,
            })]
            bool,
        )
    }
    
    let test = TestEnum::Third(true);
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = TestEnum::Third(false);
    assert!(test.validate().is_err());
}

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
fn test_option() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Option {
                String {
                    length: 5,
                }
            }
        )]
        field: Option<String>,
    }

    let test = Test {
        field: Some("12345".to_string()),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: None,
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}

#[test]
fn test_option_none() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(Option {})]
        field: Option<String>,
    }

    let test = Test {
        field: Some("12345".to_string()),
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: None,
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());
}

#[test]
fn test_option_nested() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Option {
                Option {
                    String {
                        length: 5,
                    }
                }
            }
        )]
        field: Option<Option<String>>,
    }

    let test = Test {
        field: Some(Some("12345".to_string())),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: Some(None),
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: None,
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}

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

#[test]
fn test_validate_all() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                size: 6..=8,
                sign: Positive,
                step: 2,
            }
        )]
        field1: i32,
        #[rod(
            String {
                length: 5,
            }
        )]
        field2: String,
    }
    let test = Test {
        field1: 6,
        field2: "12345".to_string(),
    };
    assert!(test.validate_all().is_ok(), "{}", test.validate_all().unwrap_err());
    let test = Test {
        field1: 5,
        field2: "123456".to_string(),
    };
    assert!(test.validate_all().is_err() && test.validate_all().unwrap_err().len() == 3, "{}", test.validate_all().unwrap_err());
}

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

#[test]
fn test_user_defined_error() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                ?"hi"
                size: 6..=8,
                sign: Positive,
                step: 2,
            },
            message: "Field must be an even number between 6 and 8"
        )]
        field: i32,
        #[rod(
            String {
                length: 5,
            },
            message: "Field must be exactly 5 characters long"
        )]
        field2: String,
    }
    let test = Test {
        field: 5,
        field2: "1234".to_string(),
    };
    let err = test.validate_all().unwrap_err();
    assert!(err.len() == 3, "{}", err);
    assert!(err.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "hi")));
    assert!(err.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "Field must be an even number between 6 and 8")));
    assert!(err.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "Field must be exactly 5 characters long")));
}

#[test]
fn test_per_validation_custom_errors() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            i32 {
                ?"int size"
                size: 6..=8,
                ?"int sign"
                sign: Negative,
                ?"int step"
                step: 2,
            }
        )]
        int_field: i32,
        #[rod(
            f64 {
                ?"float size"
                size: 2.0..=4.0,
                ?"float sign"
                sign: Negative,
                ?"float type"
                ftype: Finite,
            }
        )]
        float_field: f64,
        #[rod(
            String {
                ?"len"
                length: 5,
                ?"format"
                format: Email,
                ?"starts"
                starts_with: "Hi",
                ?"ends"
                ends_with: "!",
                ?"includes"
                includes: "abc",
            }
        )]
        string_field: String,
        #[rod(
            Literal {
                ?"literal"
                value: true,
            }
        )]
        literal_field: bool,
        #[rod(
            Option {
                ?"option"
                String {
                    ?"nested string"
                    length: 3,
                }
            }
        )]
        option_field: Option<String>,
        #[rod(
            Iterable {
                ?"iter length"
                length: 2,
                ?"iter item"
                item: String {
                    ?"iter item length"
                    length: 3,
                }
            }
        )]
        iterable_field: Vec<String>,
    }

    let test = Test {
        int_field: 5,
        float_field: f64::NAN,
        string_field: "bye".to_string(),
        literal_field: false,
        option_field: None,
        iterable_field: vec!["xx".to_string()],
    };

    let errors = test.validate_all().unwrap_err();
    assert_eq!(errors.len(), 15, "{}", errors);
    
    for expected in [
        "int size",
        "int sign",
        "int step",
        "float size",
        "float sign",
        "float type",
        "len",
        "format",
        "starts",
        "ends",
        "includes",
        "literal",
        "option",
        "iter length",
        "iter item length",
    ] {
        assert!(errors.iter().any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == expected)), "Missing expected message `{}` in errors: {}", expected, errors);
    }
}