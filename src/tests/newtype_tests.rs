//! Tests for brand/newtype structs, nested error paths, and multi-value
//! literals.

use crate::prelude::*;

// ---------------------------------------------------------------------------
// Newtype structs
// ---------------------------------------------------------------------------

#[test]
fn newtype_struct_validates_inner_field() {
    #[derive(RodValidate)]
    struct Email(#[rod(String { length: 5..=100 })] String);

    let good = Email("user@example.com".to_string());
    assert!(good.validate().is_ok(), "{}", good.validate().unwrap_err());
    assert!(good.validate_all().is_ok());

    let bad = Email("".to_string());
    let err = bad.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::String(_)), "{}", err);
}

#[test]
fn newtype_struct_custom_check() {
    #[derive(RodValidate)]
    struct Even(#[rod(i32, check = |x| x % 2 == 0)] i32);

    let good = Even(4);
    assert!(good.validate().is_ok());

    let bad = Even(3);
    let err = bad.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::CheckFailed(path) if path == "__inner"));
}

#[test]
fn newtype_struct_metadata_and_schema() {
    #[derive(RodValidate)]
    struct Port(#[rod(u16 { size: 1024..=65535 })] u16);

    let fields = Port::FIELDS;
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "__inner");

    #[cfg(feature = "serde")]
    {
        // The schema describes the inner rule, not an opaque object.
        let schema = Port::to_json_schema();
        assert_eq!(schema["type"], "integer");
        assert_eq!(schema["minimum"], 1024);
        assert_eq!(schema["maximum"], 65535);
    }

    let valid = Port(8080);
    assert!(valid.validate().is_ok(), "{}", valid.validate().unwrap_err());
    let invalid = Port(80);
    assert!(invalid.validate().is_err());
}

#[test]
fn newtype_struct_async_check() {
    use super::support::block_on;

    #[derive(RodValidate)]
    struct Session(
        #[rod(
            String { length: 3..=12 },
            async_check = |t: &String| std::future::ready(t.starts_with("tok"))
        )]
        String,
    );

    // The async closure sees the inner `String`.
    let ok = Session("tok123".to_string());
    assert!(ok.validate().is_ok(), "{}", ok.validate().unwrap_err());
    assert!(block_on(ok.validate_async()).is_ok());

    let bad = Session("zzz456".to_string());
    // Sync rules pass; the async check rejects the prefix.
    assert!(bad.validate().is_ok());
    assert!(block_on(bad.validate_async()).is_err());
}

#[test]
fn newtype_struct_delegates_to_nested_rod_type() {
    // Newtype wrapping another RodValidate type: delegation path with the
    // Nested error wrapper.
    #[derive(RodValidate)]
    struct Contact(Address2);

    #[derive(RodValidate)]
    struct Address2 {
        #[rod(String { length: 2..=5 })]
        city: String,
    }

    let bad = Contact(Address2 { city: "way-too-long".to_string() });
    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert_eq!(errors.iter().next().unwrap().path(), vec!["__inner"]);
}

#[test]
fn unit_struct_validates_trivially() {
    #[derive(RodValidate)]
    struct Marker;

    assert!(Marker.validate().is_ok());
    assert!(Marker.validate_all().is_ok());
    assert!(Marker::FIELDS.is_empty());
}

// ---------------------------------------------------------------------------
// Nested error paths
// ---------------------------------------------------------------------------

#[derive(RodValidate)]
struct Address {
    #[rod(String { length: 2..=5 })]
    city: String,
}

#[derive(RodValidate)]
struct Person {
    #[rod(String { length: 1..=10 })]
    name: String,
    // No attribute -> delegates to Address's own RodValidate.
    address: Address,
}

#[test]
fn nested_error_shows_dotted_path() {
    let bad = Person {
        name: "ab".to_string(),
        address: Address {
            city: "way-too-long-city".to_string(),
        },
    };

    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors); // only `city` fails

    let err = errors.iter().next().unwrap();
    assert_eq!(err.path(), vec!["address"], "{}", err);
    // Display renders the dotted path prefix.
    let text = format!("{}", err);
    assert!(text.starts_with("address:"), "{}", text);
}

#[test]
fn top_level_errors_have_empty_path() {
    let bad = Person {
        name: "this-name-is-way-too-long".to_string(),
        address: Address {
            city: "ok".to_string(),
        },
    };

    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(errors.iter().next().unwrap().path().is_empty());
}

#[test]
fn nested_path_survives_multiple_levels() {
    #[derive(RodValidate)]
    struct Company {
        ceo: Person, // Person { name, address: Address }
    }

    let bad = Company {
        ceo: Person {
            name: "ab".to_string(),
            address: Address {
                city: "nope-this-is-long".to_string(),
            },
        },
    };

    let errors = bad.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    // Two Nested layers accumulate both segments, outermost first.
    assert_eq!(errors.iter().next().unwrap().path(), vec!["ceo", "address"]);
    // Display joins the whole chain with dots.
    let text = format!("{}", errors.iter().next().unwrap());
    assert!(text.starts_with("ceo.address:"), "{}", text);
}

// ---------------------------------------------------------------------------
// Literal any (multi-value)
// ---------------------------------------------------------------------------

#[test]
fn literal_any_accepts_listed_values() {
    #[derive(RodValidate)]
    struct Role {
        #[rod(Literal { any: ["admin", "mod"] })]
        role: String,
    }

    assert!(Role { role: "admin".to_string() }.validate().is_ok());
    assert!(Role { role: "mod".to_string() }.validate().is_ok());

    let err = Role { role: "user".to_string() }.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::Literal(_)), "{}", err);
    let text = format!("{}", err);
    assert!(text.contains("admin"), "{}", text);
    assert!(text.contains("mod"), "{}", text);
}

#[test]
fn literal_any_paren_form_and_numbers() {
    #[derive(RodValidate)]
    struct Level {
        #[rod(Literal { any: (1, 2, 3) })]
        level: i32,
        #[rod(Literal { any: [true] })]
        on: bool,
    }

    assert!(Level { level: 2, on: true }.validate().is_ok());
    assert!(Level { level: 9, on: true }.validate().is_err());
    assert!(Level { level: 1, on: false }.validate().is_err());
}

#[test]
fn literal_any_with_custom_message() {
    #[derive(RodValidate)]
    struct Role {
        #[rod(
            Literal { any: ["admin", "mod"] },
            message: "invalid role"
        )]
        role: String,
    }

    let err = Role { role: "user".to_string() }.validate().unwrap_err();
    assert!(matches!(err, RodValidateError::UserDefined(msg) if msg == "invalid role"));
}

#[test]
fn literal_is_canonical_form() {
    #[derive(RodValidate)]
    struct Mode {
        #[rod(Literal { is: 42 })]
        answer: i32,
    }

    assert!(Mode { answer: 42 }.validate().is_ok());
    assert!(Mode { answer: 41 }.validate().is_err());
}

#[test]
fn literal_value_deprecated_still_works() {
    // `value:` is the deprecated alias of `is:`; itself it must still
    // validate correctly (the deprecation warning is emitted at compile
    // time by the derive).
    #[derive(RodValidate)]
    struct Mode {
        #[rod(Literal { value: 42 })]
        answer: i32,
    }

    assert!(Mode { answer: 42 }.validate().is_ok());
    assert!(Mode { answer: 41 }.validate().is_err());
}

#[cfg(feature = "serde")]
#[test]
fn literal_any_schema_is_enum() {
    #[derive(RodValidate)]
    struct Role {
        #[rod(Literal { any: ["admin", "mod"] })]
        role: String,
    }

    let schema = Role::to_json_schema();
    let values = schema["properties"]["role"]["enum"].as_array().unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], "admin");
    assert_eq!(values[1], "mod");
    // No `const` key on enum schemas.
    assert!(schema["properties"]["role"].get("const").is_none());
}
