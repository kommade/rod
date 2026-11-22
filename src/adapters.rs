//! Adapters for validating **untrusted input**.
//!
//! rod derives work on already-typed Rust values; these adapters close the
//! gap to Zod's `schema.safeParse(unknown)`: take raw bytes/JSON, deserialize
//! into the target type and run its validation, reporting exactly what went
//! wrong (deserialize failure **or** the collected rule violations).
//!
//! Requires the `serde` feature.
//!
//! ```rust
//! use rod_validation::prelude::*;
//!
//! #[derive(Debug, serde_derive::Deserialize, RodValidate)]
//! struct Signup {
//!     #[rod(String { length: 3..=12 })]
//!     username: String,
//!     #[rod(u8 { size: 1..=3 })]
//!     size: u8,
//! }
//!
//! let raw = br#"{ "username": "kim", "size": 2 }"#;
//! let signup: Signup = rod_validation::adapters::json::parse(raw).unwrap();
//! assert_eq!(signup.username, "kim");
//!
//! let raw = br#"{ "username": "k", "size": 9 }"#;
//! let err = rod_validation::adapters::json::parse::<Signup>(raw).unwrap_err();
//! // Rule failure, not a syntax error:
//! let rod_validation::adapters::ParseError::Validation(_) = err else { panic!("wrong kind") };
//! ```

use crate::errors::RodValidateErrorList;
use std::fmt::{Display, Formatter};

/// Failure of [`json::parse`] / [`json::parse_str`]. Exactly one kind:
///
/// - [`Deserialize`](ParseError::Deserialize): the input was not valid JSON /
///   did not match the shape of `T` (unknown fields included, when `T` opts
///   into `#[serde(deny_unknown_fields)]`).
/// - [`Validation`](ParseError::Validation): deserialization succeeded but
///   the collected rule violations failed.
#[derive(Debug)]
pub enum ParseError {
    /// Shape/JSON failure.
    Deserialize(serde_json::Error),
    /// Rule failures.
    Validation(RodValidateErrorList),
}

impl ParseError {
    /// The rule violations, if the failure was one of validation (and not
    /// of shape); `None` for deserialize failures.
    pub fn validation(&self) -> Option<&RodValidateErrorList> {
        match self {
            ParseError::Deserialize(_) => None,
            ParseError::Validation(errors) => Some(errors),
        }
    }

    /// `true` when the input could not be shaped into `T` at all
    /// (malformed JSON, type mismatch, missing/unknown field, ...).
    pub fn is_deserialize_failure(&self) -> bool {
        matches!(self, ParseError::Deserialize(_))
    }

    /// `true` when the input was shaped correctly but failed the rules.
    pub fn is_validation_failure(&self) -> bool {
        matches!(self, ParseError::Validation(_))
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Deserialize(e) => write!(f, "invalid input: {}", e),
            ParseError::Validation(errors) => write!(f, "{}", errors),
        }
    }
}

impl std::error::Error for ParseError {}

/// Everything an input could fail on. Branch on [`ParseError`]'s variants
/// (or use its `is_*` helpers) to tell shape failures from rule failures.
pub type ParseResult<T> = Result<T, ParseError>;

pub(crate) mod sealed {
    use super::{ParseError, ParseResult};

    /// Deserialize + validate one value.
    pub(crate) fn parse_one<T>(value: serde_json::Value) -> ParseResult<T>
    where
        T: serde::de::DeserializeOwned + crate::RodValidate,
    {
        let parsed: T = match serde_json::from_value(value) {
            Ok(v) => v,
            Err(e) => return Err(ParseError::Deserialize(e)),
        };
        match parsed.validate_all() {
            Ok(()) => Ok(parsed),
            Err(errors) => Err(ParseError::Validation(errors)),
        }
    }
}

/// JSON adapters: raw bytes/strings/`Value` -> validated `T`.
pub mod json {
    use super::{ParseError, ParseResult, sealed};

    /// Deserialize raw JSON bytes into `T`, then validate it.
    pub fn parse<T>(raw: &[u8]) -> ParseResult<T>
    where
        T: serde::de::DeserializeOwned + crate::RodValidate,
    {
        let value: serde_json::Value = match serde_json::from_slice(raw) {
            Ok(v) => v,
            Err(e) => return Err(ParseError::Deserialize(e)),
        };
        sealed::parse_one(value)
    }

    /// Deserialize a JSON string into `T`, then validate it.
    pub fn parse_str<T>(raw: &str) -> ParseResult<T>
    where
        T: serde::de::DeserializeOwned + crate::RodValidate,
    {
        let value: serde_json::Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => return Err(ParseError::Deserialize(e)),
        };
        sealed::parse_one(value)
    }

    /// Validate an already-parsed JSON value.
    pub fn parse_value<T>(value: serde_json::Value) -> ParseResult<T>
    where
        T: serde::de::DeserializeOwned + crate::RodValidate,
    {
        sealed::parse_one(value)
    }
}
