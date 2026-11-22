//! Tests for the derive macro's JSON Schema generation (requires the
//! `serde` feature, which pulls in `serde_json`).

use crate::prelude::*;

#[derive(RodValidate)]
struct StringRules {
    #[rod(
        String {
            length: 3..=12,
        }
    )]
    username: String,
    #[rod(
        String {
            length: 8,
        }
    )]
    password: String,
    #[rod(
        String {
            length: 2..6,
            starts_with: "ab",
            ends_with: "yz",
            includes: "cd",
        }
    )]
    codename: String,
}

#[test]
fn test_schema_string_rules() {
    let schema = StringRules::to_json_schema();

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["type"], "object");
    let username = &schema["properties"]["username"];
    assert_eq!(username["type"], "string");
    assert_eq!(username["minLength"], 3);
    assert_eq!(username["maxLength"], 12);

    let password = &schema["properties"]["password"];
    assert_eq!(password["minLength"], 8);
    assert_eq!(password["maxLength"], 8);

    // Exclusive range: `2..6` means max length is 5.
    let codename = &schema["properties"]["codename"];
    assert_eq!(codename["minLength"], 2);
    assert_eq!(codename["maxLength"], 5);
    assert_eq!(codename["x-rod-starts-with"], "ab");
    assert_eq!(codename["x-rod-ends-with"], "yz");
    assert_eq!(codename["x-rod-includes"], "cd");

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 3);
}

#[test]
fn test_schema_string_format_with_regex_feature() {
    #[cfg(feature = "regex")]
    {
        #[derive(RodValidate)]
        struct WithFormat {
            #[rod(
                String {
                    format: Email,
                }
            )]
            email: String,
        }
        let schema = WithFormat::to_json_schema();
        assert_eq!(schema["properties"]["email"]["format"], "email");

        #[derive(RodValidate)]
        struct WithNewFormats {
            #[rod(String { format: DateTime })]
            occurred_at: String,
            #[rod(String { format: Duration })]
            ttl: String,
            #[rod(String { format: Ulid })]
            ref_id: String,
        }
        let schema = WithNewFormats::to_json_schema();
        assert_eq!(schema["properties"]["occurred_at"]["format"], "date-time");
        assert_eq!(schema["properties"]["ttl"]["format"], "duration");
        assert_eq!(schema["properties"]["ref_id"]["format"], "ulid");
    }
}

#[test]
fn test_schema_number_rules() {
    #[derive(RodValidate)]
    struct Numbers {
        #[rod(
            i32 {
                size: 6..8,
                sign: Positive,
                step: 2,
            }
        )]
        times: i32,
        #[rod(
            u16 {
                size: 18..=99,
                sign: Nonnegative,
            }
        )]
        age: u16,
        #[rod(
            f64 {
                size: 2.0..=4.0,
                sign: Negative,
            }
        )]
        ratio: f64,
    }

    let schema = Numbers::to_json_schema();
    let times = &schema["properties"]["times"];
    assert_eq!(times["type"], "integer");
    assert_eq!(times["minimum"], 6);
    assert_eq!(times["exclusiveMaximum"], 8);
    assert_eq!(times["multipleOf"], 2);
    assert_eq!(times["exclusiveMinimum"], 0);

    let age = &schema["properties"]["age"];
    assert_eq!(age["type"], "integer");
    assert_eq!(age["minimum"], 18);
    assert_eq!(age["maximum"], 99);

    let ratio = &schema["properties"]["ratio"];
    assert_eq!(ratio["type"], "number");
    assert_eq!(ratio["minimum"].as_f64().unwrap(), 2.0);
    assert_eq!(ratio["maximum"].as_f64().unwrap(), 4.0);
    assert_eq!(ratio["exclusiveMaximum"], 0);
}

#[test]
fn test_schema_literal_boolean_skip() {
    #[derive(RodValidate)]
    struct Mixed {
        #[rod(
            Literal {
                is: true,
            }
        )]
        enabled: bool,
        #[rod(
            Literal {
                is: "admin",
            }
        )]
        role: String,
        #[rod(bool)]
        alive: bool,
        #[rod(skip)]
        internal: i32,
    }

    let schema = Mixed::to_json_schema();
    let enabled = &schema["properties"]["enabled"];
    assert_eq!(enabled["type"], "boolean");
    assert_eq!(enabled["const"], true);
    let role = &schema["properties"]["role"];
    assert_eq!(role["type"], "string");
    assert_eq!(role["const"], "admin");
    let alive = &schema["properties"]["alive"];
    assert_eq!(alive["type"], "boolean");

    // Skipped fields are not part of the schema at all.
    assert!(schema["properties"].get("internal").is_none());
    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 3);
    assert!(!required.iter().any(|v| v.as_str() == Some("internal")));
}

#[test]
fn test_schema_option_fields() {
    #[derive(RodValidate)]
    struct WithOptions {
        #[rod(
            Option {
                String {
                    length: 5,
                }
            }
        )]
        nickname: Option<String>,
        #[rod(Option {})]
        must_be_none: Option<String>,
    }

    let schema = WithOptions::to_json_schema();
    let nickname = &schema["properties"]["nickname"];
    assert_eq!(nickname["type"], "string");
    assert_eq!(nickname["minLength"], 5);

    let must_be_none = &schema["properties"]["must_be_none"];
    assert_eq!(must_be_none["type"], "null");

    // All fields are optional: no `required` key is emitted at all.
    assert!(schema.get("required").is_none() || schema["required"].as_array().unwrap().is_empty());
}

#[test]
fn test_schema_iterable_and_tuple() {
    #[derive(RodValidate)]
    struct Collections {
        #[rod(
            Iterable {
                item: i32 {
                    size: 6..=8,
                },
                length: 2,
            }
        )]
        scores: Vec<i32>,
        #[rod(
            Tuple (
                i32 {
                    size: 6..8,
                },
                String {
                    length: 5,
                }
            )
        )]
        pair: (i32, String),
    }

    let schema = Collections::to_json_schema();
    let scores = &schema["properties"]["scores"];
    assert_eq!(scores["type"], "array");
    assert_eq!(scores["minItems"], 2);
    assert_eq!(scores["maxItems"], 2);
    assert_eq!(scores["items"]["type"], "integer");
    assert_eq!(scores["items"]["minimum"], 6);
    assert_eq!(scores["items"]["maximum"], 8);

    let pair = &schema["properties"]["pair"];
    assert_eq!(pair["type"], "array");
    let prefix = pair["prefixItems"].as_array().unwrap();
    assert_eq!(prefix.len(), 2);
    assert_eq!(prefix[0]["type"], "integer");
    assert_eq!(prefix[0]["minimum"], 6);
    assert_eq!(prefix[0]["exclusiveMaximum"], 8);
    assert_eq!(prefix[1]["type"], "string");
    assert_eq!(prefix[1]["minLength"], 5);
}

#[test]
fn test_schema_map_field() {
    use std::collections::HashMap;

    #[derive(RodValidate)]
    struct Collections {
        #[rod(
            Map {
                key: String { length: 1..=10 },
                value: i32 { size: 1..=99 },
                length: 1..=3,
            }
        )]
        amounts: HashMap<String, i32>,
    }

    let schema = Collections::to_json_schema();
    let amounts = &schema["properties"]["amounts"];
    // Maps are JSON objects whose values carry the `value` rule; the entry
    // count becomes min/maxProperties. Key rules are runtime-only.
    assert_eq!(amounts["type"], "object");
    assert_eq!(amounts["additionalProperties"]["type"], "integer");
    assert_eq!(amounts["additionalProperties"]["minimum"], 1);
    assert_eq!(amounts["additionalProperties"]["maximum"], 99);
    assert_eq!(amounts["minProperties"], 1);
    assert_eq!(amounts["maxProperties"], 3);
}

#[test]
fn test_schema_nested_custom_struct() {
    #[derive(RodValidate)]
    struct Inner {
        #[rod(
            i32 {
                size: 1..=5,
            }
        )]
        counter: i32,
    }

    #[derive(RodValidate)]
    struct Outer {
        inner: Inner,
    }

    let schema = Outer::to_json_schema();
    // Nested RodValidate types are described as opaque objects (their rules
    // live in their own schema).
    assert_eq!(schema["properties"]["inner"]["type"], "object");
    assert_eq!(schema["required"].as_array().unwrap().len(), 1);
}

#[test]
fn test_schema_enum_one_of() {
    #[derive(RodValidate)]
    enum Shape {
        Circle(
            #[rod(
                Literal {
                    is: 0,
                }
            )]
            i32,
        ),
        Rect {
            #[rod(Literal {
                is: true,
            })]
            solid: bool,
        },
        Empty,
    }

    let schema = Shape::to_json_schema();
    let one_of = schema["oneOf"].as_array().unwrap();
    assert_eq!(one_of.len(), 3);

    // Tuple variant -> position-typed items.
    assert_eq!(one_of[0]["type"], "array");
    assert_eq!(one_of[0]["prefixItems"][0]["const"], 0);

    // Struct variant -> object.
    assert_eq!(one_of[1]["type"], "object");
    assert_eq!(one_of[1]["properties"]["solid"]["const"], true);
    assert_eq!(one_of[1]["required"].as_array().unwrap().len(), 1);

    // Unit variant -> null.
    assert_eq!(one_of[2]["type"], "null");
}

#[test]
fn test_schema_validation_test_utils_interop() {
    // The generated schema must be valid JSON that serde_json can index.
    let schema = StringRules::to_json_schema();
    assert!(schema.is_object());
    assert_eq!(schema["properties"]["username"]["type"], "string");
}
