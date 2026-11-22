//! Typed attribute rules: `NaiveDate { min, max }`, `NaiveTime { ... }`,
//! `NaiveDateTime { ... }`, `DateTime { ... }` (zoned, bounds in UTC),
//! `Uuid { version }` and `Url { scheme, host, host_suffix }` — each
//! spelled as the field's Rust type, per the keyword-matches-type doctrine.

#![cfg(any(feature = "chrono", feature = "uuid", feature = "url"))]

use crate::prelude::*;

#[cfg(feature = "chrono")]
mod datetime_rule_tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn naive_date_min_max_bounds() {
        #[derive(RodValidate)]
        struct Booking {
            #[rod(NaiveDate {
                min: "2026-01-01",
                max: "2026-12-31",
            })]
            check_in: NaiveDate,
        }

        // Inclusive bounds: the boundary values pass.
        assert!(
            Booking {
                check_in: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            }
            .validate_all()
            .is_ok()
        );
        assert!(
            Booking {
                check_in: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            }
            .validate_all()
            .is_ok()
        );

        // Below min / above max fail with typed errors.
        let errors = Booking {
            check_in: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        }
        .validate_all()
        .unwrap_err();
        assert_eq!(errors.len(), 1, "{}", errors);
        assert!(matches!(
            errors.iter().next().unwrap(),
            RodValidateError::Datetime(DatetimeValidation::Min(..))
        ));
        assert!(
            errors
                .iter()
                .next()
                .unwrap()
                .to_string()
                .contains("2026-01-01")
        );

        let errors = Booking {
            check_in: NaiveDate::from_ymd_opt(2027, 3, 3).unwrap(),
        }
        .validate_all()
        .unwrap_err();
        assert!(matches!(
            errors.iter().next().unwrap(),
            RodValidateError::Datetime(DatetimeValidation::Max(..))
        ));
        assert!(
            errors
                .iter()
                .next()
                .unwrap()
                .to_string()
                .contains("2026-12-31")
        );
    }

    #[test]
    fn naive_date_one_sided_bounds() {
        #[derive(RodValidate)]
        struct Window {
            #[rod(NaiveDate { min: "2026-06-01" })]
            start: NaiveDate,
            #[rod(NaiveDate { max: "2026-06-30" })]
            end: NaiveDate,
        }

        assert!(
            Window {
                start: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            }
            .validate_all()
            .is_ok()
        );

        assert!(
            Window {
                start: NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
                end: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            }
            .validate_all()
            .is_err()
        );
    }

    #[test]
    fn naive_time_min_max() {
        #[derive(RodValidate)]
        struct Shift {
            #[rod(NaiveTime {
                min: "07:30",
                max: "18:30:00",
            })]
            start: chrono::NaiveTime,
        }

        assert!(
            Shift {
                start: chrono::NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
            }
            .validate_all()
            .is_ok()
        );
        assert!(
            Shift {
                start: chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            }
            .validate_all()
            .is_ok()
        );
        assert!(
            Shift {
                start: chrono::NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            }
            .validate_all()
            .is_err()
        );
    }

    #[test]
    fn naive_datetime_bounds() {
        #[derive(RodValidate)]
        struct Deploy {
            #[rod(NaiveDateTime {
                min: "2026-11-01T00:00",
                max: "2026-11-30T23:59:59",
            })]
            when: chrono::NaiveDateTime,
        }

        let inside = chrono::NaiveDate::from_ymd_opt(2026, 11, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(Deploy { when: inside }.validate_all().is_ok());

        let early = chrono::NaiveDate::from_ymd_opt(2026, 10, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        let errors = Deploy { when: early }.validate_all().unwrap_err();
        assert!(matches!(
            errors.iter().next().unwrap(),
            RodValidateError::Datetime(DatetimeValidation::Min(..))
        ));
    }

    #[test]
    fn zoned_datetime_bounds_interpreted_as_utc() {
        use chrono::{FixedOffset, TimeZone, Utc};

        #[derive(RodValidate)]
        struct Event {
            #[rod(DateTime { min: "2026-01-01T00:00:00" })]
            starts_at: chrono::DateTime<chrono::Utc>,
            #[rod(DateTime { min: "2026-01-01T00:00:00" })]
            local_starts: chrono::DateTime<FixedOffset>,
        }

        // Exactly the bound (in UTC): passes for both the Utc-typed and
        // FixedOffset-typed fields (05:30+05:30 == 00:00Z).
        let event = Event {
            starts_at: Utc.from_utc_datetime(
                &chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            ),
            local_starts: FixedOffset::east_opt(5 * 3600)
                .unwrap()
                .with_ymd_and_hms(2026, 1, 1, 5, 30, 0)
                .unwrap(),
        };
        assert!(
            event.validate_all().is_ok(),
            "{}",
            event.validate_all().unwrap_err()
        );

        let before = Event {
            starts_at: Utc.from_utc_datetime(
                &chrono::NaiveDate::from_ymd_opt(2025, 12, 31)
                    .unwrap()
                    .and_hms_opt(23, 59, 59)
                    .unwrap(),
            ),
            local_starts: FixedOffset::east_opt(5 * 3600)
                .unwrap()
                .with_ymd_and_hms(2026, 1, 1, 4, 0, 0)
                .unwrap(),
        };
        assert!(before.validate_all().is_err());
    }

    #[test]
    fn datetime_rules_inside_iterable_and_map_and_option() {
        use std::collections::HashMap;

        #[derive(RodValidate)]
        struct Trip {
            #[rod(Iterable {
                item: NaiveDate { min: "2026-01-01", max: "2026-12-31" },
                length: 2,
            })]
            days: Vec<NaiveDate>,
            #[rod(Map {
                key: Skip,
                value: NaiveDate { min: "2026-01-01" },
            })]
            by_leg: HashMap<String, NaiveDate>,
            #[rod(Option { NaiveDate { max: "2027-01-01" } })]
            maybe_return: Option<NaiveDate>,
        }

        let good = Trip {
            days: vec![
                NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 2, 2).unwrap(),
            ],
            by_leg: HashMap::new(),
            maybe_return: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
        };
        assert!(good.validate_all().is_ok());

        // A failing item surfaces through the Iterable with typed error.
        let bad = Trip {
            days: vec![
                NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2027, 2, 1).unwrap(),
            ],
            by_leg: HashMap::new(),
            maybe_return: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
        };
        let errors = bad.validate_all().unwrap_err();
        assert!(matches!(
            errors.iter().next().unwrap(),
            RodValidateError::Datetime(DatetimeValidation::Max(..))
        ));

        // `Option { ... }` requires Some, then applies the bounds.
        let none = Trip {
            days: vec![
                NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 2, 2).unwrap(),
            ],
            by_leg: HashMap::new(),
            maybe_return: None,
        };
        assert!(none.validate_all().is_err());
    }

    #[test]
    fn datetime_bounds_with_custom_messages() {
        #[derive(RodValidate)]
        struct Ticketed {
            #[rod(
                NaiveDate {
                    ?"event is fully sold out for this date"
                    min: "2026-04-01",
                    max: "2026-04-30",
                }
            )]
            show: NaiveDate,
        }

        let errors = Ticketed {
            show: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        }
        .validate_all()
        .unwrap_err();
        assert!(matches!(
            errors.iter().next().unwrap(),
            RodValidateError::UserDefined(msg) if msg == "event is fully sold out for this date"
        ));
    }
}

#[cfg(feature = "uuid")]
mod uuid_rule_tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn uuid_version_rule() {
        #[derive(RodValidate)]
        struct Api {
            #[rod(Uuid { version: 4 })]
            request_id: Uuid,
        }

        assert!(
            Api {
                request_id: Uuid::new_v4()
            }
            .validate_all()
            .is_ok()
        );

        // A v1-shaped uuid fails the version check.
        let v1 = Uuid::parse_str("00000000-0000-1000-8000-000000000000").unwrap();
        let errors = Api { request_id: v1 }.validate_all().unwrap_err();
        assert_eq!(errors.len(), 1, "{}", errors);
        assert!(matches!(
            errors.iter().next().unwrap(),
            RodValidateError::Uuid(UuidValidation::Version(..))
        ));
        assert!(
            errors
                .iter()
                .next()
                .unwrap()
                .to_string()
                .contains("version 4")
        );
    }

    #[test]
    fn uuid_bare_and_custom_message() {
        #[derive(RodValidate)]
        struct Pair {
            #[rod(Uuid)]
            any_version: Uuid,
            #[rod(
                Uuid {
                    ?"must be a v4 uuid"
                    version: 4,
                }
            )]
            forced: Uuid,
        }

        // Bare Uuid: any version passes.
        let v1 = Uuid::parse_str("00000000-0000-1000-8000-000000000000").unwrap();
        assert!(
            Pair {
                any_version: v1,
                forced: Uuid::new_v4()
            }
            .validate_all()
            .is_ok()
        );

        assert!(
            Pair {
                any_version: v1,
                forced: v1
            }
            .validate_all()
            .is_err()
        );
    }
}

#[cfg(feature = "url")]
mod url_rule_tests {
    use super::*;

    #[test]
    fn url_scheme_host_suffix() {
        #[derive(RodValidate)]
        struct Link {
            #[rod(Url {
                scheme: "https",
                host_suffix: "example.com",
            })]
            href: url::Url,
        }

        assert!(
            Link {
                href: url::Url::parse("https://api.example.com/v1/things").unwrap(),
            }
            .validate_all()
            .is_ok()
        );
        assert!(
            Link {
                href: url::Url::parse("https://example.com/").unwrap(),
            }
            .validate_all()
            .is_ok()
        );

        // Wrong scheme.
        assert!(
            Link {
                href: url::Url::parse("http://api.example.com/").unwrap(),
            }
            .validate_all()
            .is_err()
        );
        // Wrong domain.
        assert!(
            Link {
                href: url::Url::parse("https://api.example.org/").unwrap(),
            }
            .validate_all()
            .is_err()
        );
    }

    #[test]
    fn url_exact_host_and_error_content() {
        #[derive(RodValidate)]
        struct Strict {
            #[rod(Url {
                scheme: "https",
                host: "example.com",
            })]
            href: url::Url,
        }

        assert!(
            Strict {
                href: url::Url::parse("https://example.com/whatever").unwrap(),
            }
            .validate_all()
            .is_ok()
        );
        // Exact host: subdomains do NOT match.
        assert!(
            Strict {
                href: url::Url::parse("https://api.example.com/").unwrap(),
            }
            .validate_all()
            .is_err()
        );

        // Hostless url (mailto) fails both the scheme and host rules.
        let errors = Strict {
            href: url::Url::parse("mailto:someone@example.com").unwrap(),
        }
        .validate_all()
        .unwrap_err();
        assert!(errors.iter().all(|e| matches!(e, RodValidateError::Url(_))));
    }
}

#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn schema_annotates_typed_rules() {
    #[derive(RodValidate)]
    struct All {
        #[rod(NaiveDate { min: "2026-01-01", max: "2026-12-31" })]
        on: chrono::NaiveDate,
        #[rod(NaiveDateTime { min: "2026-01-01T08:00" })]
        at: chrono::NaiveDateTime,
        #[rod(Uuid { version: 4 })]
        id: uuid::Uuid,
    }

    let schema = All::to_json_schema();
    let on = &schema["properties"]["on"];
    assert_eq!(on["format"], "date");
    assert_eq!(on["x-rod-min"], "2026-01-01");
    assert_eq!(on["x-rod-max"], "2026-12-31");

    let at = &schema["properties"]["at"];
    assert_eq!(at["x-rod-format"], "naive-date-time");
    assert_eq!(at["x-rod-min"], "2026-01-01T08:00");

    let id = &schema["properties"]["id"];
    assert_eq!(id["format"], "uuid");
    assert_eq!(id["x-rod-uuid-version"], 4);
}

#[cfg(all(feature = "serde", feature = "url"))]
#[test]
fn schema_annotates_url_rule() {
    #[derive(RodValidate)]
    struct WithUrl {
        #[rod(Url { scheme: "https" })]
        href: url::Url,
    }

    let schema = WithUrl::to_json_schema();
    let href = &schema["properties"]["href"];
    assert_eq!(href["format"], "uri");
    assert_eq!(href["x-rod-url-scheme"], "https");
}
