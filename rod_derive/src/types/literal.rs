use proc_macro_error::abort;
use syn::{parse::Parse, Ident, PatLit};
use quote::quote;

use super::optional_braced;


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
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = inner.parse()?;
                if ident == "value" {
                    check_already_used_attr!(value, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    value = Some(inner.parse()?);
                } else {
                    abort!(
                        ident.span(),
                        "Unknown attribute `{}`", ident
                    );
                }
                _ = inner.parse::<syn::Token![,]>();
            } else {
                abort!(
                    inner.span(),
                    "Expected an identifier"
                );
            }
        }
        if let Some(value) = value {
            Ok(RodLiteralContent { value })
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
        let ret = wrap_return(quote! {
            RodValidateError::Literal(LiteralValidation::Value(#path, #field_name.clone().to_string(), format!("to be {}", #value)))
        });
        quote! {
            if #field_name.clone() != #value {
                #ret;
            }
        }
    }
}