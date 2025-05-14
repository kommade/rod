use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitInt};
use quote::quote;


use super::{LengthOrSize, NumberSign};

/// `RodIntegerContent` is a struct that represents the content of an integer field in a Rod entity.
/// It is used to parse and validate integer attributes in the `#[rod]` attribute macro.
/// This struct includes optional fields for size, sign, and step, which are used in validation checks.
/// # Attributes
/// - `size`: An optional attribute that specifies a range for the integer to be in, or an exact value for the integer.
/// - `sign`: An optional attribute that specifies the sign of the integer, see [`NumberSign`][crate::types::NumberSign] enum.
/// - `step`: An optional attribute that specifies that the integer must be a multiple of this value.
/// # Usage
/// ```
/// use rod::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct MyEntity {
///    #[rod(
///         i32 {
///             size: 1..=10, // or size: 6
///             sign: Positive,
///             step: 2,
///         }
///     )]
///     my_integer: i32,
/// }
/// 
/// let entity = MyEntity { my_integer: 6 };
/// assert!(entity.validate().is_ok());
/// ```
pub struct RodIntegerContent {
    size: Option<LengthOrSize>,
    sign: Option<NumberSign>,
    step: Option<LitInt>,
}

impl RodIntegerContent {
    pub(crate) fn get_validations(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        let size_opt = self.size.as_ref().map(|size| size.validate_integer(field_name));
        let sign_opt = self.sign.as_ref().map(|sign| {
            let sign_check = match sign {
                NumberSign::Positive => quote!(*#field_name > 0),
                NumberSign::Negative => quote!(*#field_name < 0),
                NumberSign::Nonpositive => quote!(*#field_name <= 0),
                NumberSign::Nonnegative => quote!(*#field_name >= 0),
            };
            quote! {
                if !(#sign_check) {
                    return Err(RodValidateError::Integer(IntegerValidation::Sign(#field_name.clone().into(), #sign)));
                }
            }
        });
        let step_opt = self.step.as_ref().map(|step| {
            quote! {
                if #field_name % #step != 0 {
                    return Err(RodValidateError::Integer(IntegerValidation::Step(#field_name.clone().into(), #step.into())));
                }
            }
        });
        quote! {
            #size_opt
            #sign_opt
            #step_opt
        }
    }
}

impl Parse for RodIntegerContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut size = None;
        let mut sign = None;
        let mut step = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                if ident == "size" {
                    check_already_used_attr!(size, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    size = Some(input.parse()?);
                } else if ident == "sign" {
                    check_already_used_attr!(sign, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    sign = Some(input.parse()?);
                } else if ident == "step" {
                    check_already_used_attr!(step, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    step = Some(input.parse()?);
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

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(RodIntegerContent { size, sign, step, })
    }
}