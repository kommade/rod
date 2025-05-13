use proc_macro_error::abort;
use syn::{parse::Parse, Expr, ExprRange, Ident, LitInt};
use quote::{quote, ToTokens};

use crate::GetValidations;

use super::LengthOrSize;

pub(crate) enum IntegerSign {
    Positive,
    Negative,
    Nonpositive,
    Nonnegative,
}

impl ToTokens for IntegerSign {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = match self {
            IntegerSign::Positive => "Positive",
            IntegerSign::Negative => "Negative",
            IntegerSign::Nonpositive => "Nonpositive",
            IntegerSign::Nonnegative => "Nonnegative",
        };
        tokens.extend(quote!(#ident));
    }
}

impl Parse for IntegerSign {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Positive" => Ok(IntegerSign::Positive),
            "Negative" => Ok(IntegerSign::Negative),
            "Nonpositive" => Ok(IntegerSign::Nonpositive),
            "Nonnegative" => Ok(IntegerSign::Nonnegative),
            _ => Err(input.error("Expected `sign` to be one of Positive, Negative, Nonpositive, Nonnegative")),
        }
    }
}

pub(crate) struct RodIntegerContent {
    size: Option<LengthOrSize>,
    sign: Option<IntegerSign>,
    step: Option<LitInt>,
}

impl GetValidations for RodIntegerContent {
    fn get_validations(&self, field_name: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
        let mut validations = Vec::with_capacity(3);
        
        if let Some(size) = &self.size {
            validations.push(size.validate_integer(field_name.clone()));
        }

        if let Some(sign) = &self.sign {
            let sign_check = match sign {
                IntegerSign::Positive => quote!(#field_name > 0),
                IntegerSign::Negative => quote!(#field_name < 0),
                IntegerSign::Nonpositive => quote!(#field_name <= 0),
                IntegerSign::Nonnegative => quote!(#field_name >= 0),
            };
            validations.push(quote! {
                if !(#sign_check) {
                    return Err(RodValidateError::Integer(IntegerValidation::Sign(#field_name.into(), #sign)));
                }
            });
        }

        if let Some(step) = &self.step {
            validations.push(quote! {
                if #field_name % #step != 0 {
                    return Err(RodValidateError::Integer(IntegerValidation::Step(#field_name.into(), #step.into())));
                }
            });
        }

        validations
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