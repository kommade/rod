use quote::{format_ident, quote};
use syn::{parse::Parse, Ident, Index};

use crate::{RodAttr, RodAttrContent};

use super::optional_paren;

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, $wrap_return:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations($field_access, $wrap_return),
            )*
        }
    };
}

/// Parsed content for a tuple field attribute in `rod`.
///
/// This struct represents the parsed attributes for each element of a tuple field,
/// as specified in the derive macro input. Each tuple element can have its own
/// set of validation attributes, which are stored as `RodAttr` instances in the
/// `fields` vector.
///
/// # Usage
/// ```
/// struct Test {
///     #[rod(
///         Tuple (
///             i32 {
///                 size: 6..8,
///                 sign: Positive,
///                 step: 2,
///             },
///             i32 {
///                 size: 6..=8,
///                 sign: Positive,
///                 step: 2,
///             }
///         )
///     )]
///     field: (i32, i32),
/// };
/// ```
///
/// In the above example, `RodTupleContent` would contain two `RodAttr` entries,
/// both defining validation rules for the two `i32` elements of the tuple.
///
/// This struct is used internally by the derive macro to generate validation logic
/// for each tuple element, based on the specified attributes.
pub struct RodTupleContent {
    pub(crate) fields: Vec<RodAttr>,
}

impl Parse for RodTupleContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_paren(input)?;
        let inner = match opt {
            Some(inner) => inner,
            None => {
                return Ok(RodTupleContent { fields: Vec::new() });
            }
        };
        let fields = inner.parse_terminated(RodAttr::parse, syn::Token![,])?;
        let fields = fields.into_iter().collect();
        Ok(RodTupleContent { fields })
    }
}

impl RodTupleContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        self.fields.iter().enumerate().map(|(i, field)| {
            let i = Index::from(i);
            let subfield_name = format_ident!("{}_{}", field_name, i);
            let inner_validation = rod_content_match!(
                &field.content,
                &subfield_name,
                wrap_return,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            );
            quote! {
                let #subfield_name = &#field_name.#i;
                #inner_validation
            }
        }).collect()
    }
}