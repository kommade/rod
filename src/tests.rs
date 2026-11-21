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

/// Tests for regex feature
#[cfg(feature = "regex")]
mod regex_tests;

// Re-export all test modules for easy access
pub use string_tests::*;
pub use integer_tests::*;
pub use float_tests::*;
pub use tuple_tests::*;
pub use struct_tests::*;
pub use enum_tests::*;
pub use option_tests::*;
pub use iterable_tests::*;
pub use custom_tests::*;
pub use error_tests::*;

#[cfg(feature = "regex")]
pub use regex_tests::*;
