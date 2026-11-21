//! Tests for option validation

use crate::prelude::*;

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
fn test_option_nested_none() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(
            Option {
                Option {
                    Option {
                        String {
                            length: 5,
                        }
                    }
                }
            }
        )]
        field: Option<Option<Option<String>>>,
    }

    let test = Test {
        field: Some(Some(Some("12345".to_string()))),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: Some(Some(None)),
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: Some(None),
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: None,
    };
    assert!(test.validate().is_err(), "{}", test.validate().unwrap_err());
}
