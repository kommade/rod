use proc_macro_error::abort;
use syn::{parse::Parse, Ident, PatLit};
use quote::quote;


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
        let mut value = None;
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                if ident == "value" {
                    check_already_used_attr!(value, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    value = Some(input.parse()?);
                } else {
                    abort!(
                        ident.span(),
                        "Unknown attribute `{}`", ident
                    );
                }
                input.parse::<syn::Token![,]>()?;
            } else {
                abort!(
                    input.span(),
                    "Expected an identifier"
                );
            }
        }
        if let Some(value) = value {
            Ok(RodLiteralContent { value })
        } else {
            abort!(input.span(), "Expected a literal value")
        }
    }
}

impl RodLiteralContent {
    pub(crate) fn get_validations(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        let value = &self.value.lit;
        quote! {
            if #field_name.clone() != #value {
                return Err(RodValidateError::Literal(LiteralValidation::Value(#field_name.to_string(), format!("to be {}", #value))));
            }
        }
    }
}