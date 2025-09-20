#![allow(unused)]

pub use crate::errors::*;

pub use crate::RodValidate;

/// Doctests
/// 
/// Substruct does not implement `RodValidate`
/// ```compile_fail
/// use rod::prelude::*;
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
/// use rod::prelude::*;
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
/// use rod::prelude::*;
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
/// use rod::prelude::*;
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
/// use rod::prelude::*;
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
/// Reference to a reference
/// 
/// ```compile_fail
/// use rod::prelude::*;
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
///    field: &'static &'static i32, // This will fail to compile
/// }
/// 
/// ```
/// Check syntax that doesn't return a boolean
/// ```compile_fail
/// use rod::prelude::*;
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