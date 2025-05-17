use syn::parse::Parse;
use quote::quote;

pub struct CustomContent;

impl Parse for CustomContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![,]) || input.is_empty() {
            Ok(CustomContent)
        } else {
            Err(syn::Error::new(input.span(), "Unexpected content in #[rod(Custom)]"))
        }
    }
}

impl CustomContent {
    pub(crate) fn get_validations(&self, field_name: &syn::Ident) -> proc_macro2::TokenStream {
        quote! {
            assert_impl_rod_validate(#field_name)?;
        }
    }
}