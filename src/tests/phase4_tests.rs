//! Tests for `distinct` on iterables and `check_all`
//! multi-issue closures.

use crate::prelude::*;

// ---------------------------------------------------------------------------
// Iterable distinct
// ---------------------------------------------------------------------------

#[test]
fn distinct_accepts_unique_items() {
    #[derive(RodValidate)]
    struct Tags {
        #[rod(
            Iterable {
                item: String { length: 1..=10 },
                distinct,
            }
        )]
        tags: Vec<String>,
    }

    let good = Tags {
        tags: vec![
            "rust".to_string(),
            " validating".to_string().trim().to_string(),
            "ok".to_string(),
        ],
    };
    assert!(
        good.validate_all().is_ok(),
        "{}",
        good.validate_all().unwrap_err()
    );
}

#[test]
fn distinct_rejects_duplicates() {
    #[derive(RodValidate)]
    struct Tags {
        #[rod(
            Iterable {
                item: String { length: 1..=10 },
                distinct,
            }
        )]
        tags: Vec<String>,
    }

    let bad = Tags {
        tags: vec!["a".to_string(), "b".to_string(), "a".to_string()],
    };
    let errors = bad.validate_all().unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, RodValidateError::Iterable(IterableValidation::Distinct(path)) if *path == "tags")),
        "{}",
        errors
    );
}

#[test]
fn distinct_false_allows_duplicates() {
    #[derive(RodValidate)]
    struct Tags {
        #[rod(
            Iterable {
                item: String { length: 1..=10 },
                distinct = false,
            }
        )]
        tags: Vec<String>,
    }

    let dupes = Tags {
        tags: vec!["a".to_string(), "a".to_string()],
    };
    assert!(
        dupes.validate_all().is_ok(),
        "{}",
        dupes.validate_all().unwrap_err()
    );
}

#[test]
fn distinct_numbers() {
    #[derive(RodValidate)]
    struct Scores {
        #[rod(
            Iterable {
                item: i32 { size: 0..=100 },
                distinct,
            }
        )]
        scores: Vec<i32>,
    }

    assert!(
        Scores {
            scores: vec![1, 2, 3]
        }
        .validate_all()
        .is_ok()
    );
    assert!(Scores { scores: vec![1, 1] }.validate_all().is_err());
}

#[test]
fn distinct_with_length_combined() {
    #[derive(RodValidate)]
    struct Lottery {
        #[rod(
            Iterable {
                item: u8 { size: 1..=49 },
                length: 6,
                distinct,
            }
        )]
        numbers: Vec<u8>,
    }

    assert!(
        Lottery {
            numbers: vec![1, 5, 9, 20, 33, 49]
        }
        .validate_all()
        .is_ok()
    );
    // One duplicate and one-out-of-six: two issues.
    let err = Lottery {
        numbers: vec![1, 1, 9, 20, 33, 49],
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(err.len(), 1, "{}", err);
}

#[cfg(feature = "serde")]
#[test]
fn distinct_emits_unique_items_schema() {
    #[derive(RodValidate)]
    struct Tags {
        #[rod(
            Iterable {
                item: String { length: 1..=10 },
                distinct,
            }
        )]
        tags: Vec<String>,
    }

    let schema = Tags::to_json_schema();
    assert_eq!(schema["properties"]["tags"]["uniqueItems"], true);
}

// ---------------------------------------------------------------------------
// check_all (multi-issue collection)
// ---------------------------------------------------------------------------

#[test]
fn check_all_passes_when_empty() {
    #[derive(RodValidate)]
    struct Password {
        #[rod(
            String { length: 8..=30 },
            check_all = |p: &String| {
                let mut issues = Vec::new();
                if !p.chars().any(|c| c.is_ascii_digit()) {
                    issues.push("needs a digit");
                }
                if !p.chars().any(|c| c.is_ascii_uppercase()) {
                    issues.push("needs an uppercase letter");
                }
                issues
            }
        )]
        password: String,
    }

    let good = Password {
        password: "GoodPass1".to_string(),
    };
    assert!(
        good.validate_all().is_ok(),
        "{}",
        good.validate_all().unwrap_err()
    );
}

#[test]
fn check_all_emits_one_error_per_issue() {
    #[derive(RodValidate)]
    struct Password {
        #[rod(
            String { length: 8..=30 },
            check_all = |p: &String| {
                let mut issues = Vec::new();
                if !p.chars().any(|c| c.is_ascii_digit()) {
                    issues.push("needs a digit");
                }
                if !p.chars().any(|c| c.is_ascii_uppercase()) {
                    issues.push("needs an uppercase letter");
                }
                issues
            }
        )]
        password: String,
    }

    // Both issues fire: two UserDefined errors from ONE closure.
    let bad = Password {
        password: "lowercaseonly".to_string(),
    }; // has length, no digit, no upper
    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
    for expected in ["needs a digit", "needs an uppercase letter"] {
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, RodValidateError::UserDefined(msg) if *msg == expected)),
            "missing `{}` in {}",
            expected,
            errors
        );
    }
}

#[test]
fn check_all_only_field_delegates_to_type() {
    // A field may carry only `check_all` (no rod rule): validation first
    // delegates to the field type's own `RodValidate` impl, then the
    // multi-issue closure runs on top.
    #[derive(RodValidate)]
    struct Pair {
        #[rod(i32 { size: 0..=100 })]
        low: i32,
        #[rod(i32 { size: 0..=100 })]
        high: i32,
    }

    #[derive(RodValidate)]
    struct Range {
        #[rod(
            check_all = |p: &Pair| {
                if p.low > p.high { vec!["low must not exceed high"] } else { vec![] }
            }
        )]
        bounds: Pair,
    }

    assert!(
        Range {
            bounds: Pair { low: 1, high: 10 }
        }
        .validate_all()
        .is_ok()
    );

    // Inner rule violation AND the check_all issue: both collected.
    let err = Range {
        bounds: Pair { low: 500, high: 1 },
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(err.len(), 2, "{}", err);

    // check_all fires while the inner rules pass.
    let err = Range {
        bounds: Pair { low: 10, high: 1 },
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(err.len(), 1, "{}", err);
    assert!(
        matches!(err.iter().next().unwrap(), RodValidateError::UserDefined(msg) if msg == "low must not exceed high")
    );
}

#[test]
fn check_all_and_sync_rules_combine() {
    #[derive(RodValidate)]
    struct Signup {
        #[rod(
            String { length: 3..=20 },
            check_all = |u: &String| {
                if u.starts_with('_') { vec!["cannot start with underscore"] } else { vec![] }
            }
        )]
        username: String,
    }

    // Fails the length rule AND the check_all issue: both collected.
    let bad = Signup {
        username: "_k".to_string(),
    };
    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);

    // Passes both.
    let ok = Signup {
        username: "kim_dev".to_string(),
    };
    assert!(ok.validate_all().is_ok());
}

// NOTE: `check` + `check_all` on the same field is a compile error
// (a derive-time `abort!`), asserted by the `compile_fail` doctest on
// `RodValidate` in src/lib.rs. A proc-macro abort cannot be observed at
// runtime with `catch_unwind`: macro expansion happens while this test
// binary is compiled, not when the test runs.
