//! Typed third-party ID/url validation: `uuid::Uuid` and `url::Url` (behind
//! their features) and the std `NonZero<N>` family (no feature needed)
//! validate trivially via blanket impls — these types cannot exist in an
//! invalid state.

use crate::prelude::*;

#[cfg(feature = "uuid")]
mod uuid_tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn uuid_field_validates() {
        #[derive(RodValidate)]
        struct Api {
            #[rod(String { length: 1..=120 })]
            name: String,
            // Unattributed Uuid: delegation, always valid.
            request_id: Uuid,
        }

        let api = Api {
            name: "GetThing".to_string(),
            request_id: Uuid::new_v4(),
        };
        assert!(api.validate_all().is_ok());
        assert!(api.validate().is_ok());

        // Only the rule can fail.
        let api = Api {
            name: String::new(),
            request_id: Uuid::new_v4(),
        };
        let errors = api.validate_all().unwrap_err();
        assert_eq!(errors.len(), 1, "{}", errors);
        assert!(matches!(errors.iter().next().unwrap(), RodValidateError::String(_)));
    }

    #[test]
    fn uuid_in_groups_and_containers() {
        #[derive(RodValidate)]
        struct Msg {
            #[rod(group = "security", String { length: 8..=64 })]
            token: String,
            id: Uuid,
        }

        let msg = Msg {
            token: "tok_12345678".to_string(),
            id: Uuid::new_v4(),
        };
        assert!(msg.validate_group("security").is_ok());
        assert!(msg.validate_group("other").is_ok());

        // Containers compose: Vec of uuids and Box<Option<Uuid>>.
        #[derive(RodValidate)]
        struct Batch {
            prior_ids: Vec<Uuid>,
            parent: Option<Box<Uuid>>,
        }
        assert!(Batch {
            prior_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
            parent: Some(Box::new(Uuid::new_v4())),
        }
        .validate_all()
        .is_ok());
    }
}

#[cfg(feature = "url")]
mod url_tests {
    use super::*;

    #[test]
    fn url_field_validates() {
        #[derive(RodValidate)]
        struct Bookmark {
            #[rod(String { length: 1..=80 })]
            title: String,
            // Unattributed Url: delegation, always valid (the type cannot
            // exist unparseable).
            href: url::Url,
        }

        let bookmark = Bookmark {
            title: "docs".to_string(),
            href: url::Url::parse("https://example.invalid/path?q=1").unwrap(),
        };
        assert!(bookmark.validate_all().is_ok());

        let bookmark = Bookmark {
            title: String::new(),
            href: url::Url::parse("https://example.invalid").unwrap(),
        };
        assert!(bookmark.validate_all().is_err());
    }
}

mod nonzero_tests {
    use super::*;
    use std::num::{NonZeroI32, NonZeroU64, NonZeroU8};

    #[test]
    fn nonzero_fields_validate() {
        #[derive(RodValidate)]
        struct Recovery {
            #[rod(Option { Custom })]
            fallback: Option<Inner>,
            attempts: NonZeroU8,
            primary_page: NonZeroU64,
            ratio: NonZeroI32,
        }

        #[derive(RodValidate)]
        struct Inner {
            #[rod(u8 { size: 0..=9 })]
            digit: u8,
        }

        let recovery = Recovery {
            fallback: Some(Inner { digit: 3 }),
            attempts: NonZeroU8::new(2).unwrap(),
            primary_page: NonZeroU64::new(1).unwrap(),
            ratio: NonZeroI32::new(-4).unwrap(),
        };
        assert!(recovery.validate_all().is_ok());

        let recovery = Recovery {
            fallback: Some(Inner { digit: 30 }),
            attempts: NonZeroU8::new(2).unwrap(),
            primary_page: NonZeroU64::new(1).unwrap(),
            ratio: NonZeroI32::new(-4).unwrap(),
        };
        let errors = recovery.validate_all().unwrap_err();
        assert_eq!(errors.len(), 1, "{}", errors);
        assert_eq!(errors.iter().next().unwrap().path(), vec!["fallback"]);
    }

    #[test]
    fn nonzero_delegates_through_containers() {
        #[derive(RodValidate)]
        struct Counters {
            lanes: Vec<NonZeroU8>,
            head: Option<Box<NonZeroI32>>,
        }

        assert!(Counters {
            lanes: vec![NonZeroU8::new(1).unwrap(), NonZeroU8::new(7).unwrap()],
            head: Some(Box::new(NonZeroI32::new(30).unwrap())),
        }
        .validate_all()
        .is_ok());
    }
}

// ---------------------------------------------------------------------------
// Schema annotations (serde)
// ---------------------------------------------------------------------------

#[cfg(all(feature = "serde", any(feature = "uuid", feature = "url")))]
#[test]
fn schema_annotates_uuid_and_url_fields() {
    #[derive(RodValidate)]
    struct Ids {
        api_key: uuid::Uuid,
        #[cfg(feature = "url")]
        homepage: url::Url,
    }

    let schema = Ids::to_json_schema();
    assert_eq!(schema["properties"]["api_key"]["format"], "uuid");
    #[cfg(feature = "url")]
    assert_eq!(schema["properties"]["homepage"]["format"], "uri");
}
