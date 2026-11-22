//! Error report helpers (`flatten`/`field_errors`/...), field
//! `description`, the float `step` rule, and the fixed `ftype` codegen
//! (its `FloatValidation::Type` variant used to be missing entirely).

use crate::prelude::*;

// ---------------------------------------------------------------------------
// Report helpers on RodValidateErrorList / RodValidateError
// ---------------------------------------------------------------------------

#[test]
fn error_list_helpers_group_and_filter() {
    #[derive(RodValidate)]
    struct Address {
        #[rod(String { length: 3..=10 })]
        street: String,
    }

    #[derive(RodValidate)]
    struct Person {
        #[rod(u8 { size: 18..=120 })]
        age: u8,
        address: Option<InnerName>,
        #[rod(String { length: 2..=30 })]
        name: String,
    }

    // A nested type so delegation produces `Nested` paths.
    #[derive(RodValidate)]
    struct InnerName {
        #[rod(String { length: 5 })]
        city: String,
    }

    let person = Person {
        age: 200,
        name: "k".to_string(),
        address: Some(InnerName {
            city: "Xx".to_string(),
        }),
    };

    let errors = person.validate_all().unwrap_err();
    assert_eq!(errors.len(), 3, "{}", errors);

    // Rule errors carry no path (they are raised inside the field's own
    // validation); only DELEGATED errors get Nested paths. So `flatten`
    // yields two pathless entries plus the delegated `address` one.
    let flat = errors.flatten();
    assert_eq!(flat.len(), 3);
    assert_eq!(flat[0].0, "");
    assert_eq!(flat[1].0, "address");
    assert_eq!(flat[2].0, "");

    // field_errors: grouped by the delegated top-level field.
    let by_field = errors.field_errors();
    assert_eq!(by_field.len(), 1, "{:?}", by_field);
    assert_eq!(by_field["address"].len(), 1);

    // field: all errors of one top-level field (delegated only here).
    assert_eq!(errors.field("address").len(), 1);
    assert!(errors.field("age").is_empty());
    assert!(errors.field("nonexistent").is_empty());

    // top_level_field on the error itself.
    let single = &errors.as_slice()[1];
    assert_eq!(single.top_level_field(), Some("address"));

    // Option { Custom } delegation also groups by the outer field.
    #[derive(RodValidate)]
    struct Wrapped {
        #[rod(Option { Custom })]
        address: Option<Address>,
    }
    let wrapped = Wrapped {
        address: Some(Address {
            street: "too long a street name here".to_string(),
        }),
    };
    let wrapped_errors = wrapped.validate_all().unwrap_err();
    assert_eq!(wrapped_errors.field("address").len(), 1, "{}", wrapped_errors);
}

#[test]
fn root_errors_capture_pathless_failures() {
    #[derive(RodValidate)]
    struct Slice {
        #[rod(Iterable { item: Skip, length: 2 })]
        items: Vec<u8>,
        #[rod(
            Iterable { item: Skip },
            check_all = |items: &Vec<u8>| {
                if items.contains(&9) { vec!["do not carry nine"] } else { vec![] }
            }
        )]
        items_all: Vec<u8>,
    }

    let s = Slice {
        items: vec![1, 2, 3],
        items_all: vec![2, 9],
    };
    let errors = s.validate_all().unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
    // Iterable-length and check_all errors are raised WITHOUT a Nested
    // wrapper, so they land in root_errors and stay out of field_errors.
    assert_eq!(errors.root_errors().len(), 2, "{}", errors);
    assert!(errors.field_errors().is_empty(), "{:?}", errors.field_errors());
}

// ---------------------------------------------------------------------------
// Float `step` (multipleOf)
// ---------------------------------------------------------------------------

#[test]
fn float_step_validates_multiples() {
    #[derive(RodValidate)]
    struct Ticks {
        #[rod(f64 { step: 0.5 })]
        tick: f64,
    }

    assert!(Ticks { tick: 1.5 }.validate_all().is_ok());
    assert!(Ticks { tick: -2.0 }.validate_all().is_ok());
    assert!(Ticks { tick: 0.0 }.validate_all().is_ok());

    let errors = Ticks { tick: 1.3 }.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Float(_)
    ));
}

#[test]
fn float_step_promotes_integer_literals_and_allows_messages() {
    #[derive(RodValidate)]
    struct Increment {
        // `step: 2` on a float field is promoted to 2.0.
        #[rod(
            f64 {
                ?"must be even"
                step: 2,
            }
        )]
        amount: f64,
    }

    assert!(Increment { amount: 8.0 }.validate_all().is_ok());
    let errors = Increment { amount: 7.5 }.validate_all().unwrap_err();
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::UserDefined(msg) if msg == "must be even"
    ));
}

#[test]
fn ftype_code_produces_typed_errors() {
    // Regression guard: `ftype:` used to emit a reference to the (missing)
    // `FloatValidation::Type` variant and failed to compile at all.
    #[derive(RodValidate)]
    struct Sensors {
        #[rod(f64 { ftype: Finite })]
        reading: f64,
    }

    assert!(Sensors { reading: 1.5 }.validate_all().is_ok());

    let errors = Sensors {
        reading: f64::INFINITY,
    }
    .validate_all()
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::Float(_)
    ));
    assert!(errors.iter().next().unwrap().to_string().contains("Finite"));
}

#[cfg(feature = "serde")]
#[test]
fn float_step_and_description_export_to_schema() {
    #[derive(RodValidate)]
    struct Prices {
        #[rod(
            f64 { step: 0.01 },
            description = "Unit price with cent precision"
        )]
        min_price: f64,
    }

    let schema = Prices::to_json_schema();
    let price = &schema["properties"]["min_price"];
    assert_eq!(price["multipleOf"], 0.01);
    assert_eq!(price["description"], "Unit price with cent precision");

    // Metadata carries the description too.
    assert_eq!(Prices::FIELDS[0].description, Some("Unit price with cent precision"));
}

#[cfg(not(feature = "serde"))]
#[test]
fn description_reaches_field_meta() {
    #[derive(RodValidate)]
    struct Described {
        #[rod(
            u8 { size: 0..=9 },
            description = "number of attempts"
        )]
        attempts: u8,
    }

    assert_eq!(Described::FIELDS[0].description, Some("number of attempts"));
    assert_eq!(
        Described::FIELDS[0].name,
        "attempts"
    );
    assert_eq!(Described::FIELDS.len(), 1);
}
