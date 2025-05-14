use proc_macro_error::abort;
use syn::{parse::Parse, Ident};
use quote::{quote, ToTokens};


use super::{LengthOrSize, NumberSign};

enum FloatType {
    Nan,
    Finite,
    Infinite,
    Normal,
    Subnormal,
}

impl Parse for FloatType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let r#type = match ident.to_string().as_str() {
            "NaN" => FloatType::Nan,
            "Finite" => FloatType::Finite,
            "Infinite" => FloatType::Infinite,
            "Normal" => FloatType::Normal,
            "Subnormal" => FloatType::Subnormal,
            _ => abort!(
                ident.span(), "Unknown float type `{}`", ident;
                help = "Valid float types are: NaN, Finite, Infinite, Normal, Subnormal";
            ),
        };
        Ok(r#type)
    }
}

impl ToTokens for FloatType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = match self {
            FloatType::Nan => "NaN",
            FloatType::Finite => "Finite",
            FloatType::Infinite => "Infinite",
            FloatType::Normal => "Normal",
            FloatType::Subnormal => "Subnormal",
        };
        tokens.extend(quote!(#ident));
    }
}

/// `RodFloatContent` is a struct that represents the content of an float field in a Rod entity.
/// It is used to parse and validate float attributes in the `#[rod]` attribute macro.
/// This struct includes optional fields for size, sign, and type, which are used in validation checks.
/// # Attributes
/// - `size`: An optional attribute that specifies the a range for the float to be in, or an exact value for the float.
/// - `sign`: An optional attribute that specifies the sign of the float, see [`NumberSign`][crate::types::NumberSign] enum.
/// - `type`: An optional attribute that specifies the type of the float, see [`FloatType`][crate::types::float::FloatType] enum.
/// # Usage
/// ```
/// use rod::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct MyEntity {
///    #[rod(
///         i32 {
///             size: 1.0..=10.0, // or size: 6.0
///             sign: Positive,
///             type: Finite,
///         }
///     )]
///     my_float: i32,
/// }
/// 
/// let entity = MyEntity { my_float: 6.0 };
/// assert!(entity.validate().is_ok());
/// ```
pub struct RodFloatContent {
    size: Option<LengthOrSize>,
    sign: Option<NumberSign>,
    r#type: Option<FloatType>,
}

impl RodFloatContent {
    pub(crate) fn get_validations(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        let size_opt = self.size.as_ref().map(|size| size.validate_float(field_name));
        let sign_opt = self.sign.as_ref().map(|sign| {
            let sign_check = match sign {
                NumberSign::Positive => quote!(#field_name.is_sign_positive()),
                NumberSign::Negative => quote!(#field_name.is_sign_negative()),
                NumberSign::Nonpositive => quote!(!#field_name.is_sign_positive()),
                NumberSign::Nonnegative => quote!(!#field_name.is_sign_negative()),
            };
            quote! {
                if !(#sign_check) {
                    return Err(RodValidateError::Float(FloatValidation::Sign(#field_name.clone().into(), #sign)));
                }
            }
        });
        let type_opt = self.r#type.as_ref().map(|r#type| {
            let type_check = match r#type {
                FloatType::Nan => quote!(#field_name.is_nan()),
                FloatType::Finite => quote!(#field_name.is_finite()),
                FloatType::Infinite => quote!(#field_name.is_infinite()),
                FloatType::Normal => quote!(#field_name.is_normal()),
                FloatType::Subnormal => quote!(#field_name.is_subnormal()),
            };
            quote! {
                if !(#type_check) {
                    return Err(RodValidateError::Float(FloatValidation::Type(#field_name.clone().into(), #r#type)));
                }
            }
        });
        quote! {
            #size_opt
            #sign_opt
            #type_opt
        }
    }
}

impl Parse for RodFloatContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut size = None;
        let mut sign = None;
        let mut r#type = None;

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
                } else if ident == "type" {
                    check_already_used_attr!(r#type, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    r#type = Some(input.parse()?);
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

        Ok(RodFloatContent { size, sign, r#type })
    }
}