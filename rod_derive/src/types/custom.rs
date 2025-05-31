use proc_macro_error::abort;
use syn::parse::Parse;
use quote::quote;

use super::optional_braced;

pub struct CustomContent;

impl Parse for CustomContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner = optional_braced(input);
        if let Ok(Some(buffer)) = &inner {
            if !buffer.is_empty() {
            abort!(
                buffer.span(),
                "Custom fields do not have any attributes"
            );
            }
        }
        Ok(CustomContent)
    }
}

impl CustomContent {
    pub(crate) fn get_validations(&self, field_name: &syn::Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let ret = wrap_return(quote! { e });
        quote! {
            let assert = assert_impl_rod_validate(#field_name);
            if let Err(errs) = assert {
                for e in errs {
                    #ret;
                }
            }
        }
    }
}