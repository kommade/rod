//! Validation of std collections beyond `Vec`: sets (`HashSet`, `BTreeSet`,
//! `VecDeque`) through the existing `Iterable` rule, and key-value maps
//! (`HashMap`, `BTreeMap`) through the dedicated `Map` rule with per-entry
//! `key`/`value` paths.

use crate::prelude::*;
use std::collections::{BTreeSet, HashSet};

// ---------------------------------------------------------------------------
// Sets work through the Iterable rule (same codegen as Vec: `into_iter`,
// `len`; `distinct` is warnable-redundant but harmless)
// ---------------------------------------------------------------------------

#[test]
fn hashset_field_with_iterable_rule() {
    #[derive(RodValidate)]
    struct Tags {
        #[rod(Iterable { item: String { length: 2..=8 }, length: 1..=3 })]
        tags: HashSet<String>,
    }

    let mut ok = HashSet::new();
    ok.insert("kim".to_string());
    assert!(Tags { tags: ok }.validate_all().is_ok());

    // Length applies to sets like any collection.
    let mut too_many = HashSet::new();
    too_many.insert("aa".to_string());
    too_many.insert("bb".to_string());
    too_many.insert("cc".to_string());
    too_many.insert("dd".to_string());
    let errors = Tags { tags: too_many }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Iterable(_)
    ));

    // A failing item is caught with the item's path.
    let mut bad_item = HashSet::new();
    bad_item.insert("way-too-long-tag".to_string());
    let errors = Tags { tags: bad_item }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::String(_)
    ));
}

#[test]
fn btreeset_field_validates_items() {
    #[derive(RodValidate)]
    struct Scores {
        #[rod(Iterable { item: u8 { size: 0..=9 } })]
        scores: BTreeSet<u8>,
    }

    let good: BTreeSet<u8> = [1, 2, 3].into_iter().collect();
    assert!(Scores { scores: good }.validate_all().is_ok());

    let bad: BTreeSet<u8> = [1, 99].into_iter().collect();
    let errors = Scores { scores: bad }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Integer(_)
    ));
}

#[test]
fn vecdeque_field_with_iterable_rule() {
    use std::collections::VecDeque;

    #[derive(RodValidate)]
    struct Queue {
        #[rod(Iterable { item: u8 { size: 1..=5 }, length: 2 })]
        jobs: VecDeque<u8>,
    }

    let ok: VecDeque<u8> = [1, 5].into_iter().collect();
    assert!(Queue { jobs: ok }.validate_all().is_ok());

    let bad: VecDeque<u8> = [9].into_iter().collect();
    assert!(Queue { jobs: bad }.validate_all().is_err());

    let wrong_len: VecDeque<u8> = [1, 2, 3].into_iter().collect();
    assert!(Queue { jobs: wrong_len }.validate_all().is_err());
}

#[test]
fn set_of_nested_type_delegates_per_item() {
    #[derive(RodValidate, PartialEq, Eq, PartialOrd, Ord)]
    struct Inner {
        #[rod(u8 { size: 0..=9 })]
        digit: u8,
    }

    #[derive(RodValidate)]
    struct Badge {
        #[rod(Iterable { item: Custom })]
        set: BTreeSet<Inner>,
    }

    let mut ok = BTreeSet::new();
    ok.insert(Inner { digit: 1 });
    assert!(Badge { set: ok }.validate_all().is_ok());

    let mut bad = BTreeSet::new();
    bad.insert(Inner { digit: 42 });
    let errors = Badge { set: bad }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    // Per-item delegation keeps the item path.
    assert_eq!(errors.iter().next().unwrap().path(), vec!["item"]);
}

// ---------------------------------------------------------------------------
// Maps: the `Map` rule with key/value sides and entry-count length
// ---------------------------------------------------------------------------

#[test]
fn map_field_validates_keys_values_and_length() {
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Cart {
        #[rod(Map {
            key: String { length: 1..=10 },
            value: u32 { size: 1..=99 },
            length: 1..=2,
        })]
        quantities: HashMap<String, u32>,
    }

    let mut ok = HashMap::new();
    ok.insert("apple".to_string(), 3u32);
    assert!(Cart { quantities: ok }.validate_all().is_ok());

    // Bad value: reported on the entry's `value` path.
    let mut bad_value = HashMap::new();
    bad_value.insert("apple".to_string(), 500u32);
    let errors = Cart { quantities: bad_value }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Integer(_)
    ));

    // Bad key: reported on the entry's `key` path.
    let mut bad_key = HashMap::new();
    bad_key.insert("a-way-too-long-product-name".to_string(), 3u32);
    let errors = Cart { quantities: bad_key }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::String(_)
    ));

    // Length counts entries.
    let mut too_many = HashMap::new();
    too_many.insert("a".to_string(), 1);
    too_many.insert("b".to_string(), 2);
    too_many.insert("c".to_string(), 3);
    let errors = Cart { quantities: too_many }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Iterable(_)
    ));
}

#[test]
fn btreemap_field_with_map_rule() {
    use std::collections::BTreeMap;

    #[derive(RodValidate)]
    struct Prices {
        #[rod(HashMap { key: String { length: 1 }, value: u8 { size: 1..=9 } })]
        by_sku: BTreeMap<String, u8>,
    }

    let mut ok = BTreeMap::new();
    ok.insert("a".to_string(), 5u8);
    assert!(Prices { by_sku: ok }.validate_all().is_ok());

    let mut bad = BTreeMap::new();
    bad.insert("a".to_string(), 50u8);
    assert!(Prices { by_sku: bad }.validate_all().is_err());
}

#[test]
fn map_value_delegates_to_nested_type() {
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Line {
        #[rod(u8 { size: 0..=99 })]
        amount: u8,
    }

    #[derive(RodValidate)]
    struct Invoice {
        #[rod(HashMap { value: Line })]
        lines: HashMap<String, Line>,
    }

    let mut ok = HashMap::new();
    ok.insert("l1".to_string(), Line { amount: 7 });
    assert!(Invoice { lines: ok }.validate_all().is_ok());

    let mut bad = HashMap::new();
    bad.insert("l1".to_string(), Line { amount: 200 });
    let errors = Invoice { lines: bad }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    // Delegated entries are nested under the `value` path.
    assert_eq!(errors.iter().next().unwrap().path(), vec!["value"]);
}

#[test]
fn map_key_skip_value_rule_passes_keys_through() {
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Aliases {
        #[rod(Map { key: Skip, value: String { length: 2..=4 } })]
        aliases: HashMap<String, String>,
    }

    // Keys of any shape are fine when skipped; values are still checked.
    let mut ok = HashMap::new();
    ok.insert("!" .to_string(), "kim".to_string());
    assert!(Aliases { aliases: ok }.validate_all().is_ok());

    let mut bad = HashMap::new();
    bad.insert("".to_string(), "k".to_string());
    let errors = Aliases { aliases: bad }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(errors.iter().next().unwrap(), RodValidateError::String(_)));
}

#[test]
fn map_length_only_requires_explicit_skips() {
    use std::collections::HashMap;

    // Length-only maps must opt both sides out explicitly (`key: Skip`,
    // `value: Skip`); wrong values pass because the value side is skipped,
    // while the entry count stays enforced.
    #[derive(RodValidate)]
    struct Registry {
        #[rod(Map { key: Skip, value: Skip, length: 1 })]
        map: HashMap<String, u8>,
    }

    let mut one = HashMap::new();
    one.insert("whatever".to_string(), 200u8);
    assert!(Registry { map: one }.validate_all().is_ok());

    let mut none = HashMap::new();
    assert!(Registry { map: none }.validate_all().is_err());
}

#[test]
fn map_field_participates_in_groups() {
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Entries {
        #[rod(group = "billing", Map { key: String { length: 1..=10 }, value: u16 { size: 1..=10 } })]
        amounts: HashMap<String, u16>,
    }

    let mut amounts = HashMap::new();
    amounts.insert("day".to_string(), 99u16);

    // Outside the group: nothing runs.
    assert!(Entries { amounts: amounts.clone() }.validate_group("other").is_ok());

    // Inside the group: bad value surfaced with its entry path.
    let errors = Entries { amounts }.validate_group("billing").unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(errors.iter().next().unwrap(), RodValidateError::Integer(_)));
}

#[test]
fn map_field_with_async_check() {
    use super::support::block_on;
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Snapshot {
        #[rod(
            Map { key: Skip, value: u8 { size: 0..=9 } },
            // Real-future shape: the borrowed input must not be captured by
            // the async block itself (snapshot first), so the future stays
            // `Send` and lifetime-free.
            async_check = |m: &HashMap<String, u8>| {
                let non_empty = !m.is_empty();
                std::future::ready(non_empty)
            }
        )]
        snapshot: HashMap<String, u8>,
    }

    let mut empty = HashMap::new();
    assert!(Snapshot { snapshot: empty.clone() }.validate_all().is_ok());
    assert!(block_on(Snapshot { snapshot: empty }.validate_async()).is_err());

    let mut ok = HashMap::new();
    ok.insert("k".to_string(), 1u8);
    assert!(block_on(Snapshot { snapshot: ok }.validate_async()).is_ok());
}
