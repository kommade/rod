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

/// Shared helpers (current-thread executor)
mod support;

/// Tests for regex feature
#[cfg(feature = "regex")]
mod regex_tests;

/// Tests for JSON Schema generation (serde feature)
#[cfg(feature = "serde")]
mod json_schema_tests;

mod combinator_tests;
