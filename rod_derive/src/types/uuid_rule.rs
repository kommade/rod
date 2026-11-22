use proc_macro_error::abort;
use quote::quote;
use syn::{Ident, LitInt, LitStr, parse::Parse};

use super::user_defined_error;

/// Rule for `uuid::Uuid` fields: an optional `version` check. Bare
/// `Uuid` (no bounds) validates trivially, matching the blanket impl.
pub struct RodUuidContent {
    pub(crate) version: Option<LitInt>,
    custom_errors: [Option<LitStr>; 1],
}

impl Parse for RodUuidContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !cfg!(feature = "uuid") {
            abort!(
                input.span(), "The `Uuid` rule requires the `uuid` feature";
                help = "Enable `rod_validation/uuid` (which forwards to the derive) and depend on uuid for the field types."
            );
        }
        let opt = super::optional_braced(input)?;
        let inner = match opt {
            Some(inner) => inner,
            None => {
                return Ok(RodUuidContent {
                    version: None,
                    custom_errors: [None],
                });
            }
        };
        let mut version: Option<LitInt> = None;
        let mut custom_version: Option<LitStr> = None;
        let mut message: Option<LitStr> = None;
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = inner.parse()?;
                if ident != "version" {
                    abort!(
                        ident.span(), "Unknown attribute `{}`", ident;
                        help = "The `Uuid` rule supports `version`, e.g. `Uuid {{ version: 4 }}`."
                    );
                }
                if version.is_some() {
                    abort!(
                        ident.span(),
                        "The attribute `version` is used multiple times."
                    );
                }
                inner.parse::<syn::Token![:]>()?;
                let lit: LitInt = inner.parse()?;
                let v = lit
                    .base10_parse::<u32>()
                    .unwrap_or_else(|_| abort!(lit.span(), "Invalid uuid version"));
                if !(1..=8).contains(&v) {
                    abort!(
                        lit.span(), "Invalid uuid version {}", v;
                        help = "RFC 4122+: versions run 1 through 8 (0 would be the nil uuid, 15 is the reserved max variant)."
                    );
                }
                version = Some(lit);
                if let Some(msg) = message.take() {
                    custom_version = Some(msg);
                }
                _ = inner.parse::<syn::Token![,]>();
            } else if lookahead.peek(syn::Token![?]) {
                let _q: syn::Token![?] = inner.parse()?;
                let result: LitStr = inner.parse()?;
                message = Some(result);
            } else {
                abort!(inner.span(), "Expected `version` or `? \"message\"");
            }
        }
        Ok(RodUuidContent {
            version,
            custom_errors: [custom_version],
        })
    }
}

impl RodUuidContent {
    pub(crate) fn get_validations(
        &self,
        field_name: &Ident,
        wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        self.codegen(field_name, wrap_return, None)
    }

    pub(crate) fn get_validations_with_custom_error(
        &self,
        field_name: &Ident,
        wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        custom_error: &LitStr,
    ) -> proc_macro2::TokenStream {
        self.codegen(field_name, wrap_return, Some(custom_error))
    }

    fn codegen(
        &self,
        field_name: &Ident,
        wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        custom_error: Option<&LitStr>,
    ) -> proc_macro2::TokenStream {
        let Some(version) = self.version.as_ref() else {
            return quote! {};
        };
        let expected = version.base10_parse::<u32>().unwrap_or(0);
        let path = field_name.to_string();
        let ret = if let Some(msg) = self.custom_errors[0].as_ref().or(custom_error) {
            user_defined_error(wrap_return, msg)
        } else {
            wrap_return(quote! {
                RodValidateError::Uuid(UuidValidation::Version(#path, #expected, #field_name.get_version_num() as u32))
            })
        };
        // Nil uuids report version 0, which can never equal a required
        // version >= 1, so nils fail naturally.
        quote! {
            if ((#field_name).get_version_num() as u32) != #expected {
                #ret;
            }
        }
    }
}
