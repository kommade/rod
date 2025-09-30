use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitStr};
use quote::{quote, ToTokens};


use super::{optional_braced, user_defined_error, LengthOrSize, NumberSign};

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
/// - `ftype`: An optional attribute that specifies the type of the float, see [`FloatType`][crate::types::float::FloatType] enum.
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
///             ftype: Finite,
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
    custom_errors: [Option<LitStr>; 3], // size, sign, type
}

impl RodFloatContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let path = field_name.to_string();
        let size_opt = self.size.as_ref().map(|size| {
            if let Some(msg) = self.custom_errors[0].as_ref() {
                size.validate_float_with_custom_error(field_name, wrap_return, msg)
            } else {
                size.validate_float(field_name, wrap_return)
            }
        });
        let sign_opt = self.sign.as_ref().map(|sign| {
            let sign_check = match sign {
                NumberSign::Positive => quote!(#field_name.is_sign_positive()),
                NumberSign::Negative => quote!(#field_name.is_sign_negative()),
                NumberSign::Nonpositive => quote!(!#field_name.is_sign_positive()),
                NumberSign::Nonnegative => quote!(!#field_name.is_sign_negative()),
            };
            let ret = if let Some(msg) = self.custom_errors[1].as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Float(FloatValidation::Sign(#path, #field_name.clone().into(), #sign))
                })
            };
            quote! {
                if !(#sign_check) {
                    #ret;
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
            let ret = if let Some(msg) = self.custom_errors[2].as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Float(FloatValidation::Type(#path, #field_name.clone().into(), #r#type))
                })
            };
            quote! {
                if !(#type_check) {
                    #ret;
                }
            }
        });
        quote! {
            #size_opt
            #sign_opt
            #type_opt
        }
    }

    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        let size_opt = self.size.as_ref().map(|size| {
            if let Some(msg) = self.custom_errors[0].as_ref() {
                size.validate_float_with_custom_error(field_name, wrap_return, msg)
            } else {
                size.validate_float_with_custom_error(field_name, wrap_return, custom_error)
            }
        });
        let sign_opt = self.sign.as_ref().map(|sign| {
            let sign_check = match sign {
                NumberSign::Positive => quote!(#field_name.is_sign_positive()),
                NumberSign::Negative => quote!(#field_name.is_sign_negative()),
                NumberSign::Nonpositive => quote!(!#field_name.is_sign_positive()),
                NumberSign::Nonnegative => quote!(!#field_name.is_sign_negative()),
            };
            let ret = if let Some(msg) = self.custom_errors[1].as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                user_defined_error(wrap_return, custom_error)
            };
            quote! {
                if !(#sign_check) {
                    #ret;
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
            let ret = if let Some(msg) = self.custom_errors[2].as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                user_defined_error(wrap_return, custom_error)
            };
            quote! {
                if !(#type_check) {
                    #ret;
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
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(buffer) => buffer,
            None => return Ok(RodFloatContent {
                size: None,
                sign: None,
                r#type: None,
                custom_errors: [None, None, None],
            })
        };
        let mut size = None;
        let mut sign = None;
        let mut r#type = None;
        let mut message: Option<LitStr> = None;
        let mut custom_errors: [Option<LitStr>; 3] = [None, None, None];
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = inner.parse()?;
                if ident == "size" || ident == "range" {
                    check_already_used_attr!(size, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    size = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_errors[0] = Some(msg);
                    }
                } else if ident == "sign" {
                    check_already_used_attr!(sign, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    sign = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_errors[1] = Some(msg);
                    }
                } else if ident == "ftype" {
                    check_already_used_attr!(r#type, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    r#type = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_errors[2] = Some(msg);
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
        Ok(RodFloatContent {
            size,
            sign,
            r#type,
            custom_errors,
        })
    }
}