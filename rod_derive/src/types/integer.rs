use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitInt};
use quote::quote;


use super::{optional_braced, LengthOrSize, NumberSign};

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
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let path = field_name.to_string();
        let size_opt = self.size.as_ref().map(|size| size.validate_integer(field_name, wrap_return));
        let sign_opt = self.sign.as_ref().map(|sign| {
            let sign_check = match sign {
                NumberSign::Positive => quote!(*#field_name > 0),
                NumberSign::Negative => quote!(*#field_name < 0),
                NumberSign::Nonpositive => quote!(*#field_name <= 0),
                NumberSign::Nonnegative => quote!(*#field_name >= 0),
            };
            let ret = wrap_return(quote! {
                RodValidateError::Integer(IntegerValidation::Sign(#path, #field_name.clone().into(), #sign))
            });
            quote! {
                if !(#sign_check) {
                    #ret;
                }
            }
        });
        let step_opt = self.step.as_ref().map(|step| {
            let ret = wrap_return(quote! {
                RodValidateError::Integer(IntegerValidation::Step(#path, #field_name.clone().into(), #step.into()))
            });
            quote! {
                if #field_name % #step != 0 {
                    #ret;
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
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(buffer) => buffer,
            None => return Ok(RodIntegerContent {
                size: None,
                sign: None,
                step: None,
            }),
        };
        let mut size = None;
        let mut sign = None;
        let mut step = None;
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = inner.parse()?;
                if ident == "size" || ident == "range" {
                    check_already_used_attr!(size, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    size = Some(inner.parse()?);
                } else if ident == "sign" {
                    check_already_used_attr!(sign, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    sign = Some(inner.parse()?);
                } else if ident == "step" {
                    check_already_used_attr!(step, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    step = Some(inner.parse()?);
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
        Ok(RodIntegerContent {
            size,
            sign,
            step,
        })
    }
}