//! Tests for std-container validation (`Box`/`Rc`/`Arc`/`Vec`/`Option`
//! blanket impls, recursive types) and callback fields (fn pointers,
//! futures/tasks held in web-style structs).

use crate::prelude::*;

// ---------------------------------------------------------------------------
// Blanket impls: smart pointers, Vec, Option
// ---------------------------------------------------------------------------

#[derive(RodValidate, Debug)]
struct Inner {
    #[rod(u8 { size: 0..=9 })]
    digit: u8,
}

#[test]
fn boxed_field_delegates() {
    #[derive(RodValidate)]
    struct Wrapper {
        inner: Box<Inner>,
    }

    assert!(Wrapper { inner: Box::new(Inner { digit: 3 }) }.validate_all().is_ok());

    let errors = Wrapper {
        inner: Box::new(Inner { digit: 42 }),
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    // Delegation keeps the field path.
    assert_eq!(errors.iter().next().unwrap().path(), vec!["inner"]);
}

#[test]
fn rc_and_arc_fields_delegate() {
    use std::rc::Rc;
    use std::sync::Arc;

    #[derive(RodValidate)]
    struct Shared {
        by_rc: Rc<Inner>,
        by_arc: Arc<Inner>,
    }

    assert!(
        Shared {
            by_rc: Rc::new(Inner { digit: 1 }),
            by_arc: Arc::new(Inner { digit: 2 }),
        }
        .validate_all()
        .is_ok()
    );

    let errors = Shared {
        by_rc: Rc::new(Inner { digit: 99 }),
        by_arc: Arc::new(Inner { digit: 98 }),
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
    assert!(errors.iter().any(|e| e.path() == vec!["by_rc"]));
    assert!(errors.iter().any(|e| e.path() == vec!["by_arc"]));
}

#[test]
fn vec_field_validates_every_item() {
    #[derive(RodValidate)]
    struct Cart {
        items: Vec<Inner>,
    }

    assert!(
        Cart {
            items: vec![Inner { digit: 1 }, Inner { digit: 2 }],
        }
        .validate_all()
        .is_ok()
    );

    // Both failing items are collected.
    let errors = Cart {
        items: vec![Inner { digit: 10 }, Inner { digit: 20 }],
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);

    // validate() fails fast on the first.
    assert!(Cart {
        items: vec![Inner { digit: 10 }, Inner { digit: 20 }],
    }
    .validate()
    .is_err());
}

#[test]
fn option_field_none_is_valid_some_is_validated() {
    #[derive(RodValidate)]
    struct MaybeAddress {
        address: Option<Inner>,
    }

    assert!(MaybeAddress { address: None }.validate_all().is_ok());
    assert!(
        MaybeAddress {
            address: Some(Inner { digit: 5 }),
        }
        .validate_all()
        .is_ok()
    );
    let errors = MaybeAddress {
        address: Some(Inner { digit: 50 }),
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert_eq!(errors.iter().next().unwrap().path(), vec!["address"]);
}

#[test]
fn recursive_type_via_option_box() {
    // Tree-shaped recursion: the Rust equivalent of a lazy
    // self-referencing schema.
    #[derive(RodValidate)]
    struct Node {
        #[rod(i32 { size: 0..=100 })]
        value: i32,
        left: Option<Box<Node>>,
        right: Option<Box<Node>>,
    }

    let good = Node {
        value: 50,
        left: Some(Box::new(Node {
            value: 10,
            left: Some(Box::new(Node {
                value: 1,
                left: None,
                right: None,
            })),
            right: None,
        })),
        right: None,
    };
    assert!(good.validate_all().is_ok(), "{}", good.validate_all().unwrap_err());

    // The failing leaf is three levels deep; its error carries every hop.
    let bad = Node {
        value: 50,
        left: Some(Box::new(Node {
            value: 10,
            left: Some(Box::new(Node {
                value: 999,
                left: None,
                right: None,
            })),
            right: None,
        })),
        right: None,
    };
    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert_eq!(
        errors.iter().next().unwrap().path(),
        vec!["left", "left"]
    );
}

#[test]
fn vec_of_nested_type_group_validation() {
    // validate_group flows through the containers: the outer type is
    // group-aware, its ungrouped Vec field always runs, and each item's
    // own validate_group decides which inner fields participate.
    #[derive(RodValidate)]
    struct GroupedInner {
        #[rod(group = "billing", u8 { size: 0..=9 })]
        amount: u8,
    }

    #[derive(RodValidate)]
    struct Batch {
        rows: Vec<GroupedInner>,
        #[rod(group = "shipping", String { length: 2..=40 })]
        address: String,
    }

    let batch = Batch {
        address: "warehouse 7".to_string(),
        rows: vec![
            GroupedInner { amount: 99 },
            GroupedInner { amount: 1 },
        ],
    };

    // "other" group: rows always run (ungrouped) but each item's billing
    // field is skipped; the shipping-tagged address is skipped too.
    assert!(
        batch.validate_group("other").is_ok(),
        "{}",
        batch.validate_group("other").unwrap_err()
    );

    // "billing" group: the failing amount (99) surfaces through the Vec.
    let errors = batch.validate_group("billing").unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert_eq!(errors.iter().next().unwrap().path(), vec!["rows"]);

    // "shipping" group: address checked, inner billing skipped.
    let bad_batch = Batch {
        address: "w".to_string(),
        rows: vec![GroupedInner { amount: 99 }],
    };
    let errors = bad_batch.validate_group("shipping").unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::String(_)
    ));
}

// ---------------------------------------------------------------------------
// Callback fields (fn pointers) are auto-skipped
// ---------------------------------------------------------------------------

#[test]
fn fn_pointer_field_is_auto_skipped() {
    fn noop() -> u8 {
        7
    }

    #[derive(RodValidate)]
    struct FormHandler {
        #[rod(String { length: 3..=12 })]
        username: String,
        // Unattributed callback: compiles and validates trivially.
        on_submit: fn() -> u8,
    }

    let handler = FormHandler {
        username: "kim".to_string(),
        on_submit: noop,
    };
    assert!(handler.validate_all().is_ok(), "{}", handler.validate_all().unwrap_err());

    let errors = FormHandler {
        username: "x".to_string(),
        on_submit: noop,
    }
    .validate_all()
    .unwrap_err();
    // Only the data field is validated; the callback is ignored.
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(errors.iter().next().unwrap(), RodValidateError::String(_)));
}

#[test]
fn fn_pointer_field_excluded_from_metadata_and_schema() {
    fn noop() {}

    #[derive(RodValidate)]
    struct WithCallback {
        #[rod(u8 { size: 1..=3 })]
        tries: u8,
        on_done: fn(),
    }

    // Metadata keeps only the validated (non-callback) fields.
    assert_eq!(WithCallback::FIELDS.len(), 1);
    assert_eq!(WithCallback::FIELDS[0].name, "tries");

    #[cfg(feature = "serde")]
    {
        let schema = WithCallback::to_json_schema();
        let props = &schema["properties"];
        assert!(props.get("tries").is_some());
        assert!(props.get("on_done").is_none());
    }
}

#[test]
fn explicit_skip_still_works_on_fn_fields() {
    fn noop() -> u8 {
        1
    }

    #[derive(RodValidate)]
    struct ExplicitSkip {
        #[rod(Skip)]
        on_submit: fn() -> u8,
        #[rod(u8 { size: 1..=3 })]
        tries: u8,
    }

    assert!(
        ExplicitSkip {
            on_submit: noop,
            tries: 2,
        }
        .validate_all()
        .is_ok()
    );
    assert!(ExplicitSkip {
        on_submit: noop,
        tries: 9,
    }
    .validate_all()
    .is_err());
}

#[test]
fn newtype_wrapping_fn_pointer_validates_trivially() {
    fn noop() -> u8 {
        0
    }

    #[derive(RodValidate)]
    struct Handler(fn() -> u8);

    assert!(Handler(noop).validate_all().is_ok());
    // No inner field validated → no metadata entries.
    assert_eq!(Handler::FIELDS.len(), 0);
}

#[test]
fn enum_variant_fn_pointer_field_is_skipped() {
    fn noop() {}

    #[derive(RodValidate)]
    enum Event {
        Click {
            #[rod(u8 { size: 1..=3 })]
            button: u8,
        },
        Submit {
            #[rod(u8 { size: 1..=3 })]
            tries: u8,
            on_submit: fn(),
        },
    }

    assert!(Event::Submit { tries: 2, on_submit: noop }.validate_all().is_ok());
    let errors = Event::Submit { tries: 9, on_submit: noop }
        .validate_all()
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(Event::Click { button: 1 }.validate_all().is_ok());
}

// ---------------------------------------------------------------------------
// Futures / tasks held in structs: `#[rod(Skip)]` is the explicit tool
// (their `Pin<Box<dyn Future>>`-style types cannot be detected as
// "callback" from the type name, unlike bare fn pointers).
// ---------------------------------------------------------------------------

#[test]
fn future_field_with_skip_compiles_and_skips() {
    use std::future::Future;
    use std::pin::Pin;

    #[derive(RodValidate)]
    struct TaskHolder {
        #[rod(u8 { size: 1..=3 })]
        retries: u8,
        #[rod(Skip)]
        pending: Option<Pin<Box<dyn Future<Output = ()>>>>,
    }

    let holder = TaskHolder {
        retries: 2,
        pending: Some(Box::pin(std::future::ready(()))),
    };
    assert!(holder.validate_all().is_ok(), "{}", holder.validate_all().unwrap_err());
}

// ---------------------------------------------------------------------------
// Explicit inner-skip iterables: `Iterable { item: Skip, ... }` validates
// the collection shape and leaves the items entirely alone (skipping must
// be written out); `item: Custom` delegates per item (group-aware).
// ---------------------------------------------------------------------------

#[test]
fn item_skip_iterable_validates_only_shape() {
    #[derive(RodValidate)]
    struct Cart {
        #[rod(Iterable { item: Skip, length: 1..=3 })]
        items: Vec<Inner>,
    }

    // Length ok; the items' own rule (digit 0..=9) is never checked.
    assert!(
        Cart {
            items: vec![Inner { digit: 99 }, Inner { digit: 42 }],
        }
        .validate_all()
        .is_ok()
    );

    // Length violation surfaces as an Iterable error.
    let err = Cart { items: vec![] }.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::Iterable(_)), "{}", err);

    let errors = Cart { items: vec![] }.validate_all().unwrap_err();
    // The field path lives in the Length variant (top-level attributed
    // field, no Nested wrapping).
    assert!(
        errors.iter().any(|e| matches!(
            e,
            RodValidateError::Iterable(IterableValidation::Length(path, ..)) if *path == "items"
        )),
        "{}",
        errors
    );
}

#[test]
fn item_skip_iterable_with_distinct() {
    #[derive(RodValidate)]
    struct Lottery {
        #[rod(Iterable { item: Skip, distinct, length: 6 })]
        numbers: Vec<u8>,
    }

    assert!(
        Lottery {
            numbers: vec![1, 5, 9, 20, 33, 49],
        }
        .validate_all()
        .is_ok()
    );
    let errors = Lottery {
        numbers: vec![1, 1, 9, 20, 33, 49],
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Iterable(IterableValidation::Distinct(_))
    ));
}

#[test]
fn iterable_item_custom_delegates_per_item() {
    #[derive(RodValidate)]
    struct Cart {
        #[rod(Iterable { item: Custom, length: 1..=3 })]
        items: Vec<Inner>,
    }

    assert!(
        Cart {
            items: vec![Inner { digit: 1 }, Inner { digit: 9 }],
        }
        .validate_all()
        .is_ok()
    );

    // Both failing items collected.
    let errors = Cart {
        items: vec![Inner { digit: 10 }, Inner { digit: 20 }],
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
}

#[test]
fn iterable_item_custom_respects_group_selection() {
    #[derive(RodValidate)]
    struct GroupRow {
        #[rod(group = "billing", u8 { size: 0..=9 })]
        amount: u8,
    }

    #[derive(RodValidate)]
    struct Batch {
        #[rod(group = "billing", Iterable { item: Custom })]
        rows: Vec<GroupRow>,
    }

    let batch = Batch {
        rows: vec![GroupRow { amount: 99 }],
    };

    // validate_all: per-item delegation runs everything.
    let errors = batch.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);

    // "other": the grouped field is skipped entirely.
    assert!(batch.validate_group("other").is_ok());

    // "billing": the field runs and the group flows into each item.
    let errors = batch.validate_group("billing").unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Integer(_)
    ));
}
