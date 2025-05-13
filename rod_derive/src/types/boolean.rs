use proc_macro_error::abort;
use syn::parse::Parse;

use crate::GetValidations;

/// `RodBooleanContent` is a struct that represents the content of a boolean field in a Rod entity.
/// It is used to parse and validate boolean attributes in the `#[rod]` attribute macro.
/// The struct is empty because boolean fields do not have any specific attributes to validate.
/// To check if a boolean is true or false, use `Literal` instead. 
pub struct RodBooleanContent {}

impl Parse for RodBooleanContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !input.is_empty() {
            abort!(
                input.span(),
                "Expected no attributes for boolean fields. Use `Literal` to check if a boolean is true or false."
            )
        }
        Ok(RodBooleanContent {})
    }
}

impl GetValidations for RodBooleanContent {
    fn get_validations(&self, _field_name: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
        vec![]
    }
}