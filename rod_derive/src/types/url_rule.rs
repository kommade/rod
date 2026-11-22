use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitStr};
use quote::quote;

use super::user_defined_error;

/// Rule for `url::Url` fields: optional `scheme`, `host` (exact) and
/// `host_suffix` checks, ANDed together. Bare `Url` validates trivially,
/// matching the blanket impl.
pub struct RodUrlContent {
    scheme: Option<LitStr>,
    host: Option<LitStr>,
    host_suffix: Option<LitStr>,
    custom_errors: [Option<LitStr>; 3], // scheme, host, host_suffix
}

impl Parse for RodUrlContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !cfg!(feature = "url") {
            abort!(
                input.span(), "The `Url` rule requires the `url` feature";
                help = "Enable `rod_validation/url` (which forwards to the derive) and depend on url for the field types."
            );
        }
        let opt = super::optional_braced(input)?;
        let inner = match opt {
            Some(inner) => inner,
            None => {
                return Ok(RodUrlContent {
                    scheme: None,
                    host: None,
                    host_suffix: None,
                    custom_errors: [None, None, None],
                });
            }
        };
        let mut scheme: Option<LitStr> = None;
        let mut host: Option<LitStr> = None;
        let mut host_suffix: Option<LitStr> = None;
        let mut custom_errors: [Option<LitStr>; 3] = [None, None, None];
        let mut message: Option<LitStr> = None;
        let apply = |message: &mut Option<LitStr>, slot: &mut Option<LitStr>| {
            if let Some(msg) = message.take() {
                *slot = Some(msg);
            }
        };
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = inner.parse()?;
                let spelling = ident.to_string();
                if spelling != "scheme" && spelling != "host" && spelling != "host_suffix" {
                    abort!(
                        ident.span(), "Unknown attribute `{}`", spelling;
                        help = "The `Url` rule supports `scheme`, `host` (exact) and `host_suffix`, e.g. `Url {{ scheme: \"https\", host_suffix: \"example.com\" }}`."
                    );
                }
                inner.parse::<syn::Token![:]>()?;
                let lit: LitStr = inner.parse()?;
                if spelling == "scheme" && lit.value().is_empty() {
                    abort!(lit.span(), "The `scheme` bound cannot be empty");
                }
                if spelling == "host_suffix" && lit.value().is_empty() {
                    abort!(lit.span(), "The `host_suffix` bound cannot be empty");
                }
                match spelling.as_str() {
                    "scheme" => {
                        scheme = Some(lit);
                        apply(&mut message, &mut custom_errors[0]);
                    }
                    "host" => {
                        host = Some(lit);
                        apply(&mut message, &mut custom_errors[1]);
                    }
                    _ => {
                        host_suffix = Some(lit);
                        apply(&mut message, &mut custom_errors[2]);
                    }
                }
                _ = inner.parse::<syn::Token![,]>();
            } else if lookahead.peek(syn::Token![?]) {
                let _q: syn::Token![?] = inner.parse()?;
                let result: LitStr = inner.parse()?;
                message = Some(result);
            } else {
                abort!(inner.span(), "Expected `scheme`, `host`, `host_suffix` or `? \"message\"");
            }
        }
        Ok(RodUrlContent {
            scheme,
            host,
            host_suffix,
            custom_errors,
        })
    }
}

impl RodUrlContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        self.codegen(field_name, wrap_return, None)
    }

    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        self.codegen(field_name, wrap_return, Some(custom_error))
    }

    fn codegen(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: Option<&LitStr>) -> proc_macro2::TokenStream {
        // Interpolated as a derive-time string literal: the path slot of the
        // error variants is `&'static str`.
        let path = field_name.to_string();
        let mut code = quote! {};
        if let Some(scheme) = self.scheme.as_ref() {
            let expected = scheme.value();
            let ret = if let Some(msg) = self.custom_errors[0].as_ref().or(custom_error) {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Url(UrlValidation::Scheme(#path, (#field_name).scheme().to_string(), #expected.to_string()))
                })
            };
            code.extend(quote! {
                if (#field_name).scheme() != #expected {
                    #ret;
                }
            });
        }
        if let Some(host) = self.host.as_ref() {
            let expected = host.value();
            let ret = if let Some(msg) = self.custom_errors[1].as_ref().or(custom_error) {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Url(UrlValidation::Host(
                        #path,
                        (#field_name).host_str().unwrap_or("<none>").to_string(),
                        #expected.to_string(),
                    ))
                })
            };
            code.extend(quote! {
                if (#field_name).host_str() != Some(#expected) {
                    #ret;
                }
            });
        }
        if let Some(suffix) = self.host_suffix.as_ref() {
            let expected = suffix.value();
            let ret = if let Some(msg) = self.custom_errors[2].as_ref().or(custom_error) {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Url(UrlValidation::HostSuffix(
                        #path,
                        (#field_name).host_str().unwrap_or("<none>").to_string(),
                        #expected.to_string(),
                    ))
                })
            };
            code.extend(quote! {
                if !(#field_name).host_str().is_some_and(|rod_host| rod_host.ends_with(#expected)) {
                    #ret;
                }
            });
        }
        code
    }

    pub(crate) fn schema_exts(&self) -> Vec<(&'static str, String)> {
        let mut exts = Vec::new();
        if let Some(scheme) = &self.scheme {
            exts.push(("x-rod-url-scheme", scheme.value()));
        }
        if let Some(host) = &self.host {
            exts.push(("x-rod-url-host", host.value()));
        }
        if let Some(suffix) = &self.host_suffix {
            exts.push(("x-rod-url-host-suffix", suffix.value()));
        }
        exts
    }
}
