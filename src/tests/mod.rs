//! Comprehensive tests for rod_validation crate
//!
//! This module contains all unit tests for the rod_validation library,
//! organized into logical submodules for better maintainability.

#![allow(unused)]

use crate::prelude::*;

/// Tests for basic string validation
mod string_tests;

/// Tests for integer validation
mod integer_tests;

/// Tests for float validation
mod float_tests;

/// Tests for tuple validation
mod tuple_tests;

/// Tests for struct validation
mod struct_tests;

/// Tests for enum validation
mod enum_tests;

/// Tests for option validation
mod option_tests;

/// Tests for iterable validation
mod iterable_tests;

/// Tests for custom validation
mod custom_tests;

/// Tests for error handling
mod error_tests;

/// Tests for validation groups, metadata, metrics and async validation
mod group_tests;

/// Tests for `async_check` fields and `AsyncValidator`
mod async_tests;

/// Tests for newtype structs, nested error paths and multi-value literals
mod newtype_tests;

/// Tests for parse/sanitize transforms (RodParse)
mod parse_tests;

/// Tests for distinct iterables and check_all multi-issue closures
mod phase4_tests;

/// Tests for std-container impls (Box/Rc/Arc/Vec/Option), recursive types
/// and callback fields (fn pointers, held futures)
mod phase5_tests;

/// Tests for sets/VecDeque via `Iterable` and key-value maps via `Map`
/// (keys, values, entry-count length, per-entry paths, groups, async)
mod collection_tests;

/// Tests for the untrusted-input adapters (serde)
#[cfg(feature = "serde")]
mod adapter_tests;

/// Shared helpers (current-thread executor)
mod support;

/// Tests for regex feature
#[cfg(feature = "regex")]
mod regex_tests;

/// Tests for JSON Schema generation (serde feature)
#[cfg(feature = "serde")]
mod json_schema_tests;

mod combinator_tests;

/// Error report helpers, field descriptions, float step and ftype
mod error_report_tests;

/// Path-aware multi-issue checks (`check_all_paths`/`async_check_all_paths`)
mod check_paths_tests;

/// Typed chrono datetimes (blanket impls, schema hints, serde bridge)
#[cfg(feature = "chrono")]
mod chrono_tests;

/// Typed uuid/url (feature-gated) and std NonZero* validation
mod typed_id_tests;

/// Typed attribute rules: datetime bounds, uuid version, url checks
#[cfg(any(feature = "chrono", feature = "uuid", feature = "url"))]
mod typed_rule_tests;

/// Async validation against a real Tokio runtime (timers, spawned tasks,
/// concurrent `join!`) — the plain async tests only use instantly-ready
/// futures with the no-op-waker executor.
mod tokio_tests;
