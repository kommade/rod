//! Golden-fixture tests for the untrusted-input adapters (requires `serde`).

#![cfg(feature = "serde")]

use crate::prelude::*;
use rod_validation::adapters::{ParseError, json};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, RodValidate)]
struct Signup {
    #[rod(String { length: 3..=12 })]
    username: String,
    #[rod(u8 { size: 1..=3 })]
    attempts: u8,
}

#[test]
fn happy_path_parses_and_validates() {
    let raw = br#"{ "username": "kim", "attempts": 2 }"#;
    let signup: Signup = json::parse(raw).unwrap();
    assert_eq!(signup.username, "kim");
    assert_eq!(signup.attempts, 2);
}

#[test]
fn malformed_json_reports_deserialize_error() {
    let raw = br#"{ "username": "kim", "attempts":"#;
    let err = json::parse::<Signup>(raw).unwrap_err();
    assert!(
        matches!(err, ParseError::Deserialize(_)),
        "expected deserialize failure: {}",
        err
    );
    assert!(err.is_deserialize_failure());
    assert!(!err.is_validation_failure());
    assert!(err.validation().is_none());
}

#[test]
fn type_mismatch_reports_deserialize_error() {
    let raw = br#"{ "username": "kim", "attempts": "not-a-number" }"#;
    let err = json::parse::<Signup>(raw).unwrap_err();
    assert!(matches!(err, ParseError::Deserialize(_)), "{}", err);
}

#[test]
fn missing_field_reports_deserialize_error() {
    let raw = br#"{ "username": "kim" }"#;
    let err = json::parse::<Signup>(raw).unwrap_err();
    assert!(matches!(err, ParseError::Deserialize(_)), "{}", err);
}

#[test]
fn rule_violation_reports_validation_error() {
    let raw = br#"{ "username": "k", "attempts": 9 }"#;
    let err = json::parse::<Signup>(raw).unwrap_err();
    let ParseError::Validation(errors) = &err else {
        panic!("expected validation failure: {}", err);
    };
    assert_eq!(errors.len(), 2, "{}", errors); // length + size violations
    assert!(err.is_validation_failure());
    assert!(!err.is_deserialize_failure());
    assert!(err.validation().is_some());
}

#[test]
fn parse_str_roundtrip() {
    let raw = r#"{ "username": "lena", "attempts": 1 }"#;
    let signup: Signup = json::parse_str(raw).unwrap();
    assert_eq!(signup.username, "lena");

    let err = json::parse_str::<Signup>("not json at all").unwrap_err();
    assert!(matches!(err, ParseError::Deserialize(_)));
}

#[test]
fn parse_value_entrypoint() {
    let value = serde_json::json!({ "username": "kim", "attempts": 3 });
    let signup: Signup = json::parse_value(value).unwrap();
    assert_eq!(signup.attempts, 3);
}

#[test]
fn invalid_but_wellformed_still_reports_validation() {
    // Well-formed JSON of the right shape: only rules fail.
    let raw = br#"{ "username": "kim", "attempts": 0 }"#; // 0 < 1 rule
    let err = json::parse::<Signup>(raw).unwrap_err();
    assert!(
        matches!(err, ParseError::Validation(_)),
        "expected validation failure: {}",
        err
    );
    let errors = err.validation().unwrap();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Integer(_)
    ));
}

#[test]
fn newtype_through_bridge() {
    #[derive(Debug, Deserialize, RodValidate)]
    struct Port(#[rod(u16 { size: 1024..=65535 })] u16);

    let p: Port = json::parse(br#"8080"#).unwrap();
    assert_eq!(p.0, 8080);

    let err = json::parse::<Port>(br#"80"#).unwrap_err();
    assert!(matches!(err, ParseError::Validation(_)), "{}", err);
}

#[test]
fn deny_unknown_fields_pattern() {
    // Strict mode: serde's own attribute rejects unknown keys at the
    // deserialize stage, before rod rules run.
    #[derive(Debug, Deserialize, RodValidate)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        #[rod(u8 { size: 1..=3 })]
        n: u8,
    }

    let err = json::parse::<Strict>(br#"{ "n": 2, "extra": true }"#).unwrap_err();
    // unknown key -> shape error
    assert!(matches!(err, ParseError::Deserialize(_)), "{}", err);
    assert!(err.validation().is_none());

    let ok: Strict = json::parse(br#"{ "n": 2 }"#).unwrap();
    assert_eq!(ok.n, 2);
}

#[test]
fn error_display_is_actionable() {
    let raw = br#"{ "username": "k", "attempts": 9 }"#;
    let err = json::parse::<Signup>(raw).unwrap_err();
    let text = format!("{}", err);
    // The validation list Display includes the field names.
    assert!(text.contains("username"), "{}", text);
}
