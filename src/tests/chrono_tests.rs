//! Typed datetime support: chrono's `NaiveDate`/`NaiveTime`/`NaiveDateTime`
//! /`DateTime<Tz>`/`Duration` validate trivially behind the `chrono`
//! feature, so structs can hold datetimes next to validated fields without
//! a manual impl. Constraint checks stay at the wire boundary via string
//! formats.

#![cfg(feature = "chrono")]

use crate::prelude::*;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

#[test]
fn chrono_fields_validate_trivially() {
    #[derive(RodValidate)]
    struct Booking {
        #[rod(u8 { size: 1..=3 })]
        nights: u8,
        // Unattributed chrono fields: delegation, always valid.
        check_in: NaiveDate,
        breakfast_at: NaiveTime,
        check_in_at: NaiveDateTime,
    }

    let booking = Booking {
        nights: 2,
        check_in: NaiveDate::from_ymd_opt(2026, 11, 22).unwrap(),
        breakfast_at: NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
        check_in_at: NaiveDate::from_ymd_opt(2026, 11, 22)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap(),
    };
    assert!(booking.validate_all().is_ok());
    assert!(booking.validate().is_ok());

    // The datetime fields never fail; only the rule can.
    let booking = Booking {
        nights: 9,
        check_in: NaiveDate::from_ymd_opt(2026, 11, 22).unwrap(),
        breakfast_at: NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
        check_in_at: NaiveDate::from_ymd_opt(2026, 11, 22)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap(),
    };
    let errors = booking.validate_all().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(errors.iter().next().unwrap(), RodValidateError::Integer(_)));
}

#[test]
fn zoned_datetimes_validate_for_any_timezone() {
    #[derive(RodValidate)]
    struct Event {
        starts_at_utc: DateTime<chrono::Utc>,
        ends_at_fixed: DateTime<FixedOffset>,
    }

    let utc = DateTime::from_timestamp(1_799_000_000, 0).unwrap();
    let fixed = FixedOffset::east_opt(3_600)
        .unwrap()
        .with_ymd_and_hms(2026, 11, 22, 10, 0, 0)
        .unwrap();
    assert!(Event {
        starts_at_utc: utc,
        ends_at_fixed: fixed,
    }
    .validate_all()
    .is_ok());
}

#[test]
fn chrono_duration_field_is_valid() {
    #[derive(RodValidate)]
    struct Timer {
        #[rod(u8 { size: 1..=5 })]
        lanes: u8,
        ttl: chrono::Duration,
    }

    assert!(Timer {
        lanes: 2,
        ttl: chrono::Duration::seconds(90),
    }
    .validate_all()
    .is_ok());
}

#[test]
fn chrono_field_participates_in_groups_and_containers() {
    #[derive(RodValidate)]
    struct Schedule {
        #[rod(group = "window", u8 { size: 1..=24 })]
        window_hours: u8,
        // Ungrouped chrono fields run under every group selection.
        opening: NaiveDateTime,
        // The container blanket impls compose around chrono types (Vec of
        // them; maps would need an explicit `Map` rule - no blanket).
        by_day: Vec<NaiveDate>,
    }

    let opening = NaiveDate::from_ymd_opt(2026, 11, 22)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let schedule = Schedule {
        window_hours: 8,
        opening,
        by_day: vec![NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()],
    };
    assert!(schedule.validate_group("window").is_ok());
    assert!(schedule.validate_group("other").is_ok());

    // The container blanket impls compose around chrono types.
    #[derive(RodValidate)]
    struct Recursive {
        parent: Option<Box<Recursive>>,
        date: NaiveDate,
    }
    assert!(Recursive {
        parent: Some(Box::new(Recursive { parent: None, date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() })),
        date: NaiveDate::from_ymd_opt(2026, 2, 2).unwrap(),
    }
    .validate_all()
    .is_ok());
}

#[test]
fn annotated_datetimes_honor_the_option_rule() {
    // `Option { Custom }` delegates the inner datetime and makes `None`
    // invalid, exactly like the other custom RodValidate types.
    #[derive(RodValidate)]
    struct Draft {
        #[rod(Option { Custom })]
        published_at: Option<DateTime<chrono::Utc>>,
    }

    let published = Draft {
        published_at: Some(DateTime::from_timestamp(1_799_000_000, 0).unwrap()),
    };
    assert!(published.validate_all().is_ok());

    // None is invalid under the annotated form (field required).
    assert!(Draft { published_at: None }.validate_all().is_err());
}

#[cfg(feature = "serde")]
#[test]
fn json_bridge_parses_datetimes_and_reports_stages() {
    use crate::adapters::json;

    #[derive(Debug, serde_derive::Deserialize, RodValidate)]
    struct Event {
        #[rod(String { length: 1..=40 })]
        title: String,
        // Unattributed: delegation, `None` accepted.
        scheduled_at: Option<DateTime<chrono::Utc>>,
    }

    // Happy path: the timestamp deserializes (chrono serde, RFC 3339) and
    // the rules pass.
    let parsed: Event = json::parse(br#"{ "title": "launch", "scheduled_at": "2026-11-22T10:00:00Z" }"#).unwrap();
    assert!(parsed.validate_all().is_ok());

    // Well-formed JSON, broken rules -> Validation stage.
    let err = json::parse::<Event>(br#"{ "title": "", "scheduled_at": null }"#).unwrap_err();
    assert!(err.is_validation_failure());

    // Bad timestamp -> Deserialize stage (chrono's serde rejects it).
    let err = json::parse::<Event>(br#"{ "title": "x", "scheduled_at": "not-a-date" }"#).unwrap_err();
    assert!(err.is_deserialize_failure());
}

#[cfg(all(feature = "chrono", feature = "serde"))]
#[test]
fn schema_annotates_datetime_fields() {
    #[derive(RodValidate)]
    struct WithDatetimes {
        on: NaiveDate,
        at: NaiveTime,
        exact: NaiveDateTime,
        zoned: DateTime<chrono::Utc>,
    }

    let schema = WithDatetimes::to_json_schema();
    assert_eq!(schema["properties"]["on"]["format"], "date");
    assert_eq!(schema["properties"]["at"]["format"], "time");
    assert_eq!(schema["properties"]["zoned"]["format"], "date-time");
    // NaiveDateTime is not RFC 3339: hinted via the x-rod extension rather
    // than the (stricter) standard keyword.
    assert_eq!(
        schema["properties"]["exact"]["x-rod-format"],
        "naive-date-time"
    );
    assert!(schema["properties"]["exact"].get("format").is_none());
}
