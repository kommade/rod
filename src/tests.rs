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
fn test_string_length_literals() {
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
            Tuple {
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
            }
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
            Tuple {
                i32 {
                    size: 6..8,
                    sign: Positive,
                    step: 2,
                },
                Tuple {
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
                }
            }
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