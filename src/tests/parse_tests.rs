//! Tests for Phase-3 parse/sanitize/transforms (Zod `schema.parse`).

use crate::prelude::*;

#[test]
fn trim_runs_before_validation() {
    #[derive(RodValidate, Debug)]
    struct Profile {
        #[rod(String { length: 3..=7, trim })]
        name: String,
    }

    // Without trim the 9-char (padded) value would fail the 3..=7 rule.
    let parsed = Profile {
        name: "  kim    ".to_string(),
    }
    .parse()
    .unwrap();
    assert_eq!(parsed.name, "kim"); // trailing spaces gone

    let err = Profile {
        name: "this-is-way-too-long".to_string(),
    }
    .parse()
    .unwrap_err();
    assert_eq!(err.len(), 1, "{}", err);
    assert!(matches!(
        err.iter().next().unwrap(),
        RodValidateError::String(_)
    ));
}

#[test]
fn trim_explicit_true_equals_bare_flag() {
    #[derive(RodValidate)]
    struct A {
        #[rod(String { length: 3..=7, trim = true })]
        name: String,
    }
    let parsed = A {
        name: "  kim  ".to_string(),
    }
    .parse()
    .unwrap();
    assert_eq!(parsed.name, "kim");
}

#[test]
fn no_transforms_sanitizes_to_self() {
    #[derive(RodValidate)]
    struct Plain {
        #[rod(String { length: 1..=5 })]
        name: String,
    }

    let original = Plain {
        name: "kim".to_string(),
    };
    let parsed = original.parse().unwrap();
    assert_eq!(parsed.name, "kim");
}

#[test]
fn parse_reports_all_rule_violations() {
    #[derive(RodValidate, Debug)]
    struct Form {
        #[rod(String { length: 3..=7, trim })]
        name: String,
        #[rod(u8 { size: 1..=3 })]
        tries: u8,
    }

    let err = Form {
        name: "k".to_string(),
        tries: 9,
    }
    .parse()
    .unwrap_err();
    assert_eq!(err.len(), 2, "{}", err);
}

#[test]
fn sanitize_is_usable_without_validation() {
    // `.sanitize()` alone transforms without running rules — for pipelines
    // that normalize first and validate later.
    #[derive(RodValidate, Debug)]
    struct Shout {
        #[rod(
            String { length: 1..=30, trim },
            check = |s: &String| !s.is_empty()
        )]
        text: String,
    }

    let clean = Shout {
        text: "  hi  ".to_string(),
    }
    .sanitize();
    assert_eq!(clean.text, "hi");

    // The original value is consumed, so the pipeline stays linear.
    let again = Shout {
        text: "\tx\t".to_string(),
    }
    .sanitize();
    assert_eq!(again.text, "x");
}

#[cfg(feature = "serde")]
#[test]
fn nested_parse_through_bridge() {
    #[derive(Debug, serde_derive::Deserialize, RodValidate)]
    struct Input {
        #[rod(String { length: 3..=10, trim })]
        name: String,
    }

    // The bridge deserializes verbatim (no implicit transforms) and
    // validates the raw value: the padded name passes 3..=10.
    let input: Input = rod_validation::adapters::json::parse(br#"{ "name": "  kim  " }"#).unwrap();
    assert_eq!(input.name, "  kim  ");
    assert!(input.validate().is_ok());

    // Only `parse()` applies the trim transform.
    let parsed = input.parse().unwrap();
    assert_eq!(parsed.name, "kim");

    // The other direction: raw value passes the bridge (5 chars), but the
    // trimmed value fails, proving rules run against the transformed value.
    let input: Input = rod_validation::adapters::json::parse(br#"{ "name": "  al  " }"#).unwrap();
    let err = input.parse().unwrap_err();
    assert_eq!(err.len(), 1, "{}", err);
    assert!(matches!(
        err.iter().next().unwrap(),
        RodValidateError::String(_)
    ));
}
