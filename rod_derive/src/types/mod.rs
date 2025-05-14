use proc_macro_error::abort;
use syn::{parse::Parse, ExprRange, Ident, LitInt, Token};
use quote::{quote, ToTokens};

macro_rules! check_already_used_attr {
    ($attr:ident, $span:expr) => {
        if $attr.is_some() {
            proc_macro_error::emit_warning!(
                $span, "The attribute `{}` is used multiple times. The last time it was specified will take precedence.", stringify!($attr)
            );
        }
    };
}

pub(crate) enum LengthOrSize {
    Exact(LitInt),
    Range(ExprRange),
}

impl Parse for LengthOrSize {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek2(Token![..]) {
            let range: ExprRange = input.parse()?;
            Ok(LengthOrSize::Range(range))
        } else if input.peek(LitInt) {
            let length: LitInt = input.parse()?;
            Ok(LengthOrSize::Exact(length))
        } else {
            abort!(input.span(), "Expected a number or a range")
        }
    }
}

impl LengthOrSize {
    pub(crate) fn validate_string(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        match self {
            LengthOrSize::Exact(exact) => {
                quote! {
                    if #field_name.len() != #exact {
                        return Err(RodValidateError::String(StringValidation::Length(#field_name.to_string(), format!("to be exactly {}", #exact))));
                    }
                }
            }
            LengthOrSize::Range(range) => {
                quote! {
                    if !(#range).contains(&#field_name.len()) {
                        return Err(RodValidateError::String(StringValidation::Length(#field_name.to_string(), format!("to be in the range {:?}", #range))));
                    }
                }
            }
        }
    }
    pub(crate) fn validate_integer(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        match self {
            LengthOrSize::Exact(exact) => {
                quote! {
                    if #field_name != #exact {
                        return Err(RodValidateError::Integer(IntegerValidation::Size(#field_name.clone().into(), format!("to be exactly {}", #exact))));
                    }
                }
            }
            LengthOrSize::Range(range) => {
                quote! {
                    if !(#range).contains(#field_name) {
                        return Err(RodValidateError::Integer(IntegerValidation::Size(#field_name.clone().into(), format!("to be in the range {:?}", #range))));
                    }
                }
            }
        }
    }
    pub(crate) fn validate_float(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        match self {
            LengthOrSize::Exact(exact) => {
                quote! {
                    if #field_name != #exact as f64 {
                        return Err(RodValidateError::Float(FloatValidation::Size(#field_name.clone().into(), format!("to be exactly {}", #exact))));
                    }
                }
            }
            LengthOrSize::Range(range) => {
                quote! {
                    if !(#range).contains(#field_name) {
                        return Err(RodValidateError::Float(FloatValidation::Size(#field_name.clone().into(), format!("to be in the range {:?}", #range))));
                    }
                }
            }
        }
    }
}

/// `IntegerSign` is an enum that represents the sign of an integer.
/// It is used to specify whether the integer should be positive, negative, nonpositive, or nonnegative.
pub(crate) enum NumberSign {
    Positive,
    Negative,
    Nonpositive,
    Nonnegative,
}

impl ToTokens for NumberSign {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = match self {
            NumberSign::Positive => "Positive",
            NumberSign::Negative => "Negative",
            NumberSign::Nonpositive => "Nonpositive",
            NumberSign::Nonnegative => "Nonnegative",
        };
        tokens.extend(quote!(#ident));
    }
}

impl Parse for NumberSign {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Positive" => Ok(NumberSign::Positive),
            "Negative" => Ok(NumberSign::Negative),
            "Nonpositive" => Ok(NumberSign::Nonpositive),
            "Nonnegative" => Ok(NumberSign::Nonnegative),
            _ => Err(input.error("Expected `sign` to be one of Positive, Negative, Nonpositive, Nonnegative")),
        }
    }
}

mod string;
pub use string::RodStringContent;

mod integer;
pub use integer::RodIntegerContent;

mod literal;
pub use literal::RodLiteralContent;

mod boolean;
pub use boolean::RodBooleanContent;

mod option;
pub use option::RodOptionContent;

mod float;
pub use float::RodFloatContent;

mod tuple;
pub use tuple::RodTupleContent;