//! Path-aware multi-issue checks: `check_all_paths` /
//! `async_check_all_paths` emit `Nested(field, Nested(path, UserDefined))`
//! per (path, message) pair, so issues group under the FIELD in
//! `path()`/`field_errors()` even when the closure names a sub-path.

use crate::prelude::*;

#[test]
fn check_all_paths_nests_issues_under_the_field() {
    #[derive(RodValidate)]
    struct Cart {
        #[rod(
            Iterable { item: u8 { size: 0..=9 } },
            check_all_paths = |items: &Vec<u8>| {
                let mut issues = Vec::new();
                if items.contains(&5) {
                    issues.push(("items", "five is not allowed"));
                }
                if items.len() > 3 {
                    issues.push(("items", "too many entries"));
                }
                issues
            }
        )]
        rows: Vec<u8>,
    }

    // No issues: fully valid, despite a passing rule mix.
    assert!(Cart { rows: vec![1, 2, 3] }.validate_all().is_ok());

    // Two path issues, collected (validate_all semantics).
    let errors = Cart {
        rows: vec![5, 9, 1, 2],
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
    for e in errors.iter() {
        // Field prefix + closure path; both identified as UserDefined.
        assert_eq!(e.path(), vec!["rows", "items"], "{}", e);
        assert!(matches!(e, RodValidateError::Nested(_, _)));
    }

    // flatten carries the dotted path.
    let flat = errors.flatten();
    assert!(flat.iter().all(|(p, _)| p == "rows.items"), "{:?}", flat);

    // field_errors groups under the field (a bare path would leak).
    let by_field = errors.field_errors();
    assert_eq!(by_field.len(), 1, "{:?}", by_field);
    assert_eq!(by_field["rows"].len(), 2);
    assert!(errors.root_errors().is_empty());

    // top_level_field points at the field.
    assert_eq!(
        errors.iter().next().unwrap().top_level_field(),
        Some("rows")
    );
}

#[test]
fn check_all_paths_works_on_plain_fields() {
    #[derive(RodValidate)]
    struct Slug {
        #[rod(
            String { length: 1..=10 },
            check_all_paths = |s: &String| {
                if s.starts_with('_') {
                    vec![("value", "leading underscore discouraged")]
                } else {
                    vec![]
                }
            }
        )]
        tag: String,
    }

    assert!(Slug { tag: "ok".to_string() }.validate_all().is_ok());

    let errors = Slug {
        tag: "_bad".to_string(),
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    // Layer-by-layer shape check, then content via flatten:
    let e = errors.iter().next().unwrap();
    let nested = match e {
        RodValidateError::Nested(_, nested) => nested,
        other => panic!("unexpected outer layer: {:?}", other),
    };
    match nested.as_ref() {
        RodValidateError::Nested(_, leaf) => match leaf.as_ref() {
            RodValidateError::UserDefined(_) => {}
            other => panic!("unexpected leaf: {:?}", other),
        },
        other => panic!("unexpected middle layer: {:?}", other),
    }
    let flat = errors.flatten();
    assert_eq!(flat[0].0, "tag.value");
    assert_eq!(flat[0].1, "tag.value: leading underscore discouraged");
}

#[test]
fn check_all_paths_coexists_with_rules_and_custom_errors() {
    #[derive(RodValidate)]
    struct Payment {
        #[rod(
            u8 { size: 1..=100, ?"out of range" },
            check_all_paths = |amount: &u8| {
                if amount % 10 != 0 {
                    vec![("cents", "amount must be a multiple of ten")]
                } else {
                    vec![]
                }
            }
        )]
        amount: u8,
    }

    // Rule failure only.
    let errors = Payment { amount: 200 }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    // Rule errors are pathless (no Nested wrap).
    assert_eq!(errors.iter().next().unwrap().path(), Vec::<&str>::new());

    // Path issue only.
    let errors = Payment { amount: 33 }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert_eq!(errors.iter().next().unwrap().path(), vec!["amount", "cents"]);
}

#[test]
fn async_check_all_paths_nests_issues() {
    use super::support::block_on;
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Snapshot {
        #[rod(
            Map { key: Skip, value: u8 { size: 0..=9 } },
            // Snapshot to owned before `async move` (Send + lifetime-free).
            async_check_all_paths = |m: &HashMap<String, u8>| {
                let m = m.clone();
                async move {
                    let mut issues = Vec::new();
                    if m.values().any(|v| *v > 5) {
                        issues.push(("values", "cap exceeded"));
                    }
                    if m.len() > 2 {
                        issues.push(("entries", "too many"));
                    }
                    issues
                }
            }
        )]
        snapshot: HashMap<String, u8>,
    }

    let mut quiet = HashMap::new();
    quiet.insert("a".to_string(), 1u8);
    // Sync methods run the rules but ignore async checks entirely.
    assert!(Snapshot { snapshot: quiet.clone() }.validate_all().is_ok());

    let mut noisy = HashMap::new();
    noisy.insert("a".to_string(), 9u8);
    noisy.insert("b".to_string(), 8u8);
    noisy.insert("c".to_string(), 7u8);
    let errors = block_on(Snapshot { snapshot: noisy }.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
    let mut paths: Vec<_> = errors.iter().map(|e| e.path()).collect();
    paths.sort();
    assert_eq!(paths[0], vec!["snapshot", "entries"]);
    assert_eq!(paths[1], vec!["snapshot", "values"]);

    // The pathed issues survive into the report helpers.
    let by_field = errors.field_errors();
    assert_eq!(by_field.len(), 1, "{:?}", by_field);
    assert_eq!(by_field["snapshot"].len(), 2);
}
