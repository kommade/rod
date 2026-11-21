//! Tests for the `RodValidateExt` combinators and the `ValidationPipeline` builder.

use crate::prelude::*;

#[derive(RodValidate, Clone)]
struct Code {
    #[rod(String { length: 5 })]
    secret: String,
}

fn failing(c: &Code) -> Result<(), RodValidateError> {
    let _ = c;
    Err(RodValidateError::CheckFailed("never passes"))
}

#[test]
fn test_validate_any() {
    let valid = Code { secret: "12345".to_string() };
    let invalid = Code { secret: "123".to_string() };

    // A single passing check is enough.
    assert!(valid.validate_any(&[|c: &Code| c.validate()]).is_ok());
    // All checks fail -> an error is reported.
    let err = invalid.validate_any(&[|c: &Code| c.validate()]).unwrap_err();
    assert!(matches!(err, RodValidateError::String(_)));
}

#[test]
fn test_validate_one_exactly_one() {
    let good = Code { secret: "abcde".to_string() };

    // `validate` passes, the named check fails -> exactly one passes.
    let mixed: &[fn(&Code) -> Result<(), RodValidateError>] =
        &[|c: &Code| c.validate(), failing];
    assert!(good.clone().validate_one(mixed).is_ok());

    // Both pass -> XOR violation.
    let both: &[fn(&Code) -> Result<(), RodValidateError>] =
        &[|c: &Code| c.validate(), |c: &Code| c.validate()];
    assert!(good.validate_one(both).is_err());

    // None passes -> the last error is reported.
    let bad = Code { secret: "123".to_string() };
    let none: &[fn(&Code) -> Result<(), RodValidateError>] =
        &[|c: &Code| c.validate(), failing];
    let err = bad.validate_one(none).unwrap_err();
    assert!(matches!(err, RodValidateError::CheckFailed("never passes")));
}

#[test]
fn test_validate_if_and_unless() {
    let valid = Code { secret: "12345".to_string() };
    let invalid = Code { secret: "123".to_string() };

    // validate_if: runs only when the condition holds.
    assert!(valid.clone().validate_if(true).is_ok());
    assert!(invalid.clone().validate_if(true).is_err());
    assert!(invalid.clone().validate_if(false).is_ok());

    // validate_unless: runs only when the condition does *not* hold.
    assert!(valid.clone().validate_unless(false).is_ok());
    assert!(invalid.clone().validate_unless(false).is_err());
    assert!(invalid.validate_unless(true).is_ok());
}

#[test]
fn test_validate_if_matches_form_semantics() {
    // A draft is only checked at submit time.
    let draft = Code { secret: "abc".to_string() }; // too short, but still a draft
    assert!(draft.clone().validate_if(false).is_ok());

    let submitted = Code { secret: "abc".to_string() };
    assert!(submitted.validate_if(true).is_err());
}

#[test]
fn test_validate_with_cross_field() {
    #[derive(RodValidate)]
    struct Range {
        #[rod(i32 { size: 0.. })]
        low: i32,
        #[rod(i32 { size: 0.. })]
        high: i32,
    }

    let ok = Range { low: 1, high: 10 };
    assert!(ok
        .validate_with(|r| {
            if r.low <= r.high {
                Ok(())
            } else {
                Err(RodValidateError::UserDefined("`low` must not exceed `high`".into()))
            }
        })
        .is_ok());

    // Both per-field rules pass, but the cross-field rule fails.
    let reversed = Range { low: 10, high: 1 };
    assert!(reversed.validate_all().is_ok());
    let err = reversed
        .validate_with(|r| {
            if r.low <= r.high {
                Ok(())
            } else {
                Err(RodValidateError::UserDefined("`low` must not exceed `high`".into()))
            }
        })
        .unwrap_err();
    assert!(matches!(err, RodValidateError::UserDefined(msg) if msg == "`low` must not exceed `high`"));
}

#[test]
fn test_pipeline_collects_errors() {
    let valid = Code { secret: "12345".to_string() };
    let result = valid.pipeline().step(|c| c.validate()).result();
    assert!(result.is_ok());

    let invalid = Code { secret: "123".to_string() };
    let errors = invalid.clone().pipeline()
        .step(|c| c.validate())          // contributes one error
        .step_all(|c| c.validate_all())  // contributes the same list again
        .all_errors();
    assert_eq!(errors.len(), 2, "{}", errors.len());

    // `result` reports only the first error, like `validate`.
    let first = invalid.pipeline().step(|c| c.validate()).result();
    assert!(first.is_err());
}

#[test]
fn test_validation_test_utils() {
    let valid = Code { secret: "12345".to_string() };
    assert!(valid.passes_validation());
    assert!(!valid.fails_validation());
    assert!(valid.first_validation_error().is_none());
}
