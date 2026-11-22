#![allow(unused)]

pub use crate::errors::*;
pub use crate::combinators::*;
pub use crate::{RodFieldMeta, RodMetadata, RodValidate};

/// Doctests
/// 
/// Substruct does not implement `RodValidate`
/// ```compile_fail
/// use rod_validation::prelude::*;
///
/// struct DoesNotImplementRodValidate {
///     field: String,
/// }
///
/// #[derive(RodValidate)] // This will fail to compile
/// struct Test {
///     #[rod(
///         String {
///             length: 5..=10,
///         }
///     )]
///     field1: String,
///     field2: DoesNotImplementRodValidate,
/// }
/// ```
/// 
/// Wrongly nested Options
/// ```compile_fail
/// use rod_validation::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct Test {
///     #[rod(
///         Option {
///             Option {
///                 String {
///                     length: 5,
///                 }
///             }
///         }
///     )]
///     field1: Option<String>,
/// }
/// ```
/// 
/// Option doesn't contain the correct type
/// ```compile_fail
/// use rod_validation::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct Test {
///    #[rod(
///        Option {
///           String {
///                length: 5,
///            }
///        }
///    )]
///    field1: Option<i32>,
/// }
/// ```
/// 
/// Tuple doesn't contain the correct type
/// ```compile_fail
/// use rod_validation::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct Test {
///     #[rod(
///         Tuple (
///             i32 {
///                 size: 6..8,
///                 sign: Positive,
///                 step: 2,
///             },
///             Tuple (
///                 i32 {
///                     size: 6..=8,
///                     sign: Positive,
///                     step: 2,
///                 },
///                 i32 {
///                     size: 6..=8,
///                     sign: Positive,
///                     step: 2,
///                 }
///             )
///         )
///     )]
///     field: (i32, (i32, u8)),
/// }
/// ```
/// 
/// Wrongly nested Tuples
/// ```compile_fail
/// use rod_validation::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct Test {
///     #[rod(
///         Tuple (
///             i32 {
///                 size: 6..8,
///                 sign: Positive,
///                 step: 2,
///             },
///                 i32 {
///                     size: 6..=8,
///                     sign: Positive,
///                     step: 2,
///                 },
///                 i32 {
///                     size: 6..=8,
///                     sign: Positive,
///                     step: 2,
///                 }
///         )
///     )]
///     field: (i32, (i32, i32)),
/// }
/// ```
/// 
/// Reference to a reference (supported at any depth since 0.4.0)
///
/// ```rust
/// use rod_validation::prelude::*;
///
/// #[derive(RodValidate)]
/// struct Test {
///    #[rod(
///        i32 {
///            size: 6..=8,
///            sign: Positive,
///            step: 2,
///        }
///    )]
///    field: &'static &'static i32,
/// }
///
/// static INNER: i32 = 6;
/// static REF: &'static i32 = &INNER;
///
/// let test = Test { field: &REF };
/// assert!(test.validate().is_ok());
/// assert!(test.validate_all().is_ok());
/// ```
///
/// Check syntax that doesn't return a boolean
/// ```compile_fail
/// use rod_validation::prelude::*;
/// #[derive(RodValidate)]
/// struct Test {
///     #[rod(
///         i32 {
///             size: 6..=8,
///             sign: Positive,
///         },
///         check = |x| {
///             "not a boolean"
///         }
///     )]
///     field: i32,
/// }
/// ```
pub use rod_derive::RodValidate;