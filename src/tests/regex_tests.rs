//! Tests for regex feature

use crate::prelude::*;

#[test]
fn test_string_email_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            format: Email,
        })]
        field: String,
    }

    let test = Test {
        field: "test@example.com".to_string(),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: "invalid-email".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_url_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            format: Url,
        })]
        field: String,
    }

    let test = Test {
        field: "https://example.com".to_string(),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: "not-a-url".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_uuid_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            format: Uuid,
        })]
        field: String,
    }

    let test = Test {
        field: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: "not-a-uuid".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_ipv4_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            format: Ipv4,
        })]
        field: String,
    }

    let test = Test {
        field: "192.168.1.1".to_string(),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: "not-an-ip".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_custom_regex() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            format: r"^[A-Z]{3}$",
        })]
        field: String,
    }

    let test = Test {
        field: "ABC".to_string(),
    };
    assert!(test.validate().is_ok(), "{}", test.validate().unwrap_err());

    let test = Test {
        field: "ABCD".to_string(),
    };
    assert!(test.validate().is_err());
}

#[test]
fn test_string_format_with_custom_error() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String {
            ?"custom format error"
            format: Email,
        })]
        field: String,
    }

    let test = Test {
        field: "invalid".to_string(),
    };
    let err = test.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::UserDefined(msg) if msg == "custom format error"));
}
