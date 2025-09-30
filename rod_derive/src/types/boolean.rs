use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitStr};
use quote::quote;

use super::optional_braced;


/// `RodBooleanContent` is a struct that represents the content of a boolean field in a Rod entity.
/// It is used to parse and validate boolean attributes in the `#[rod]` attribute macro.
/// The struct is empty because boolean fields do not have any specific attributes to validate.
/// To check if a boolean is true or false, use `Literal` instead. 
pub struct RodBooleanContent {}

impl Parse for RodBooleanContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner = optional_braced(input);
        if let Ok(Some(buffer)) = &inner {
            if !buffer.is_empty() {
                abort!(
                    buffer.span(),
                    "Boolean fields do not have any attributes. If you want to check if a boolean is true or false, use `Literal` instead."
                );
            }
        }
        Ok(RodBooleanContent {})
    }
}

impl RodBooleanContent {
    pub(crate) fn get_validations(&self, _field_name: &Ident, _wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        quote! {}
    }
    pub(crate) fn get_validations_with_custom_error(&self, _field_name: &Ident, _wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, _custom_error: &LitStr) -> proc_macro2::TokenStream {
        quote! {}
    }
}