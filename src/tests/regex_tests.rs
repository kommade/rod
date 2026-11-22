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

// ---------------------------------------------------------------------------
// The extended built-in formats
// ---------------------------------------------------------------------------

#[test]
fn test_string_date_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String { format: Date })]
        field: String,
    }

    assert!(Test { field: "2026-11-22".into() }.validate().is_ok());
    // Month > 12 and day > 31 are rejected.
    assert!(Test { field: "2026-13-22".into() }.validate().is_err());
    assert!(Test { field: "2026-11-00".into() }.validate().is_err());
    // Full timestamps are dates? No — date-only discipline.
    assert!(Test { field: "2026-11-22T10:00:00Z".into() }.validate().is_err());
}

#[test]
fn test_string_time_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String { format: Time })]
        field: String,
    }

    assert!(Test { field: "10:30".into() }.validate().is_ok());
    assert!(Test { field: "10:30:59".into() }.validate().is_ok());
    assert!(Test { field: "10:30:59.123Z".into() }.validate().is_ok());
    assert!(Test { field: "23:59:59+05:30".into() }.validate().is_ok());
    // Out-of-range or malformed components.
    assert!(Test { field: "24:00".into() }.validate().is_err());
    assert!(Test { field: "10:60".into() }.validate().is_err());
    assert!(Test { field: "2026-11-22".into() }.validate().is_err());
}

#[test]
fn test_string_duration_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String { format: Duration })]
        field: String,
    }

    assert!(Test { field: "P3Y".into() }.validate().is_ok());
    assert!(Test { field: "P3Y6M4D".into() }.validate().is_ok());
    assert!(Test { field: "PT12H30M5.5S".into() }.validate().is_ok());
    assert!(Test { field: "P1D".into() }.validate().is_ok());
    // Empty designators, missing P, time before date components.
    assert!(Test { field: "3Y".into() }.validate().is_err());
    assert!(Test { field: "PT".into() }.validate().is_err());
    assert!(Test { field: "P".into() }.validate().is_err());
}

#[test]
fn test_string_base64_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String { format: Base64 })]
        field: String,
    }

    assert!(Test { field: "dGVzdA==".into() }.validate().is_ok()); // "test"
    assert!(Test { field: "dGVzdA".into() }.validate().is_ok()); // unpadded len%4==0? "dGVzdA" is 6 chars -> padded group + 2 leftover
    assert!(Test { field: "".into() }.validate().is_ok()); // empty payload
    // Invalid: '+' ok but '=' mid-payload, whitespace, wrong groups.
    assert!(Test { field: "dG VzdA".into() }.validate().is_err());
    assert!(Test { field: "!!!!".into() }.validate().is_err());
}

#[test]
fn test_string_ulid_format() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String { format: Ulid })]
        field: String,
    }

    assert!(Test { field: "01ARZ3NDEKTSV4RRFFQ69G5FAV".into() }.validate().is_ok());
    assert!(Test { field: "01arz3ndektsv4rrffq69g5fav".into() }.validate().is_ok());
    // 25 chars, and excluded glyphs (I, L, O, U).
    assert!(Test { field: "01ARZ3NDEKTSV4RRFFQ69G5FA".into() }.validate().is_err());
    assert!(Test { field: "01ARZ3NDEKTSV4RRFFQ69G5FAI".into() }.validate().is_err());
}

#[test]
fn test_string_cuid_formats() {
    #[derive(RodValidate)]
    struct Test {
        #[rod(String { format: Cuid })]
        v1: String,
        #[rod(String { format: Cuid2 })]
        v2: String,
    }

    // cuid v1: `c` + 8+ body chars.
    assert!(Test {
        v1: "clx9u1abc0000abcdefghijk".into(),
        v2: "kb2f9z3lpq0rmn8tjv4wd1e6".into(),
    }.validate().is_ok(), "cuid ok case");

    assert!(Test {
        v1: "x0000abcdefghijk".into(), // missing leading c
        v2: "kb2f9z3lpq0rmn8tjv4wd1e6".into(),
    }.validate().is_err());
    assert!(Test {
        v1: "clx9u1abc0000abcdefghijk".into(),
        v2: "tooshort".into(), // < 24 chars
    }.validate().is_err());
}

