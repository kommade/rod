use proc_macro_error::abort;
use syn::{parse::Parse, LitStr};
use quote::quote;

use super::optional_braced;

/// Represents the content for a `#[rod(skip)]` field attribute.
///
/// This struct is used as a marker to indicate that a field should be skipped
/// during validation or processing. It does not contain any data or attributes,
/// and will emit a compile error if any attributes are provided.
/// 
/// # Usage
/// ```
/// use rod::prelude::*;
/// #[derive(RodValidate)]
/// struct MyEntity {
///     #[rod(skip)]
///     my_field: String, // This field will be skipped during validation
///     #[rod(Skip {})] // This will also work, but no attributes are allowed
///     another_field: i32, // This field will also be skipped
/// }
/// let entity = MyEntity {
///     my_field: "Hello".to_string(),
///     another_field: 42,
/// };
/// assert!(entity.validate().is_ok());
/// ```
pub struct RodSkipContent;

impl Parse for RodSkipContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner = optional_braced(input);
        if let Ok(Some(buffer)) = &inner {
            if !buffer.is_empty() {
                abort!(
                    input.span(),
                    "Skip fields do not have any attributes"
                );
            }
        }
        Ok(RodSkipContent {})
    }
}

impl RodSkipContent {
    pub(crate) fn get_validations(&self, _field_name: &syn::Ident, _wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        quote! {}
    }
    pub(crate) fn get_validations_with_custom_error(&self, _field_name: &syn::Ident, _wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, _custom_error: &LitStr) -> proc_macro2::TokenStream {
        quote! {}
    }
}