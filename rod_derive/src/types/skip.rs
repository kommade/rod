use syn::parse::Parse;
use quote::quote;

pub struct RodSkipContent;

impl Parse for RodSkipContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![,]) || input.is_empty() {
            Ok(RodSkipContent)
        } else {
            Err(syn::Error::new(input.span(), "Unexpected content in #[rod(Skip)]"))
        }
    }
}

impl RodSkipContent {
    pub(crate) fn get_validations(&self, _field_name: &syn::Ident) -> proc_macro2::TokenStream {
        quote! {
            // Skip validation for this field
        }
    }
}