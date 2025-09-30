use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitStr, PatLit};
use quote::quote;

use super::{optional_braced, user_defined_error};


/// `RodLiteralContent` is a struct that represents the content of a literal field in a Rod entity.
/// It is used to parse and validate literal attributes in the `#[rod]` attribute macro.
/// This struct includes a single field `value`, which is used to check if the literal value of the field matches the expected value.
/// # Attributes
/// - `value`: A required attribute that specifies the expected literal value of the field.
/// # Usage
/// ```
/// use rod::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct MyEntity {
///   #[rod(
///        literal {
///           value: 42,
///        }
///   )]
///   my_field: i32,
/// }
/// 
/// let entity = MyEntity { my_field: 42 };
/// assert!(entity.validate().is_ok());
/// ```
pub struct RodLiteralContent {
    value: PatLit,
    custom_error: Option<LitStr>,
}

impl Parse for RodLiteralContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(buffer) => buffer,
            None => {
                abort!(
                    input.span(),
                    "Must specify a literal value using `value: <literal>` inside the `literal` attribute.";
                    help = "Example: `#[rod(Literal { value: 42 })]`"
                )
            }
        };
    let mut value = None;
    let mut message: Option<LitStr> = None;
    let mut custom_error: Option<LitStr> = None;
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = inner.parse()?;
                if ident == "value" {
                    check_already_used_attr!(value, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    value = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_error = Some(msg);
                    }
                } else {
                    abort!(
                        ident.span(),
                        "Unknown attribute `{}`", ident
                    );
                }
                _ = inner.parse::<syn::Token![,]>();
            } else if lookahead.peek(syn::Token![?]) {
                let _q: syn::Token![?] = inner.parse()?;
                let result: LitStr = inner.parse()?;
                message = Some(result);
            } else {
                abort!(
                    inner.span(),
                    "Expected an identifier"
                );
            }
        }
        if let Some(value) = value {
            let custom_error = custom_error.or(message);
            Ok(RodLiteralContent { value, custom_error })
        } else {
            abort!(
                input.span(),
                "Must specify a literal value using `value: <literal>` inside the `literal` attribute.";
                help = "Example: `#[rod(Literal { value: 42 })]`"
            )
        }
    }
}

impl RodLiteralContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let path = field_name.to_string();
        let value = &self.value.lit;
        let ret = if let Some(msg) = self.custom_error.as_ref() {
            user_defined_error(wrap_return, msg)
        } else {
            wrap_return(quote! {
                RodValidateError::Literal(LiteralValidation::Value(#path, #field_name.clone().to_string(), format!("to be {}", #value)))
            })
        };
        quote! {
            if #field_name.clone() != #value {
                #ret;
            }
        }
    }
    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        let value = &self.value.lit;
        let ret = if let Some(msg) = self.custom_error.as_ref() {
            user_defined_error(wrap_return, msg)
        } else {
            user_defined_error(wrap_return, custom_error)
        };
        quote! {
            if #field_name.clone() != #value {
                #ret;
            }
        }
    }
}