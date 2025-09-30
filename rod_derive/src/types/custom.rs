use proc_macro_error::abort;
use syn::{parse::Parse, LitStr};
use quote::quote;

use super::{optional_braced, user_defined_error};

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
    pub(crate) fn get_validations_with_custom_error(&self, field_name: &syn::Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        let ret = user_defined_error(wrap_return, custom_error);
        quote! {
            let assert = assert_impl_rod_validate(#field_name);
            if let Err(_errs) = assert {
                #ret;
            }
        }
    }
}