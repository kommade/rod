use proc_macro_error::abort;
use quote::{quote, ToTokens};

use syn::{parse::Parse, Ident, LitStr, Token};

use crate::GetValidations;

use super::LengthOrSize;

#[cfg(feature = "regex")]
mod regex_literals {
    pub(crate) const EMAIL_REGEX: &str = r#"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])"#;
    pub(crate) const URL_REGEX: &str = r#"^[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&//=]*)$"#;
    pub(crate) const UUID_REGEX: &str = r#"(?i:^[0-9a-f]{8}-[0-9a-f]{4}-[0-5][0-9a-f]{3}-[089ab][0-9a-f]{3}-[0-9a-f]{12}$)"#;
    pub(crate) const IPV4_REGEX: &str = r#"^(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"#;
    pub(crate) const IPV6_REGEX: &str = r#"^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$"#;
    pub(crate) const DATETIME_REGEX: &str = r#"^(?:\d{4})-(?:\d{2})-(?:\d{2})T(?:\d{2}):(?:\d{2}):(?:\d{2}(?:\.\d*)?)(?:(?:-(?:\d{2}):(?:\d{2})|Z)?)$"#;
}

#[cfg(feature = "regex")]
pub(crate) enum StringFormat {
    Regex(LitStr),
    Email,
    Url,
    Uuid,
    Ipv4,
    Ipv6,
    DateTime,
}

#[cfg(feature = "regex")]
impl ToTokens for StringFormat {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            StringFormat::Regex(lit_str) => tokens.extend(quote!(#lit_str)),
            StringFormat::Email => tokens.extend(quote!("Email")),
            StringFormat::Url => tokens.extend(quote!("Url")),
            StringFormat::Uuid => tokens.extend(quote!("Uuid")),
            StringFormat::Ipv4 => tokens.extend(quote!("Ipv4")),
            StringFormat::Ipv6 => tokens.extend(quote!("Ipv6")),
            StringFormat::DateTime => tokens.extend(quote!("DateTime")),
        }
    }
}

#[cfg(feature = "regex")]
impl Parse for StringFormat {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            let format: LitStr = input.parse()?;
            Ok(StringFormat::Regex(format))
        } else if lookahead.peek(Ident) {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "Email" => Ok(StringFormat::Email),
                "Url" => Ok(StringFormat::Url),
                "Uuid" => Ok(StringFormat::Uuid),
                "Ipv4" => Ok(StringFormat::Ipv4),
                "Ipv6" => Ok(StringFormat::Ipv6),
                "DateTime" => Ok(StringFormat::DateTime),
                _ => abort!(ident.span(), "Expected `format` to be one of Email, Url, Uuid, Ipv4, Ipv6, DateTime or a regex string"),
            }
        } else {
            abort!(input.span(), "Expected identifier or string literal for attribute `format`");
        }
    }
}

pub(crate) struct RodStringContent {
    length: Option<LengthOrSize>,
    #[cfg(feature = "regex")]
    format: Option<StringFormat>,
}

impl GetValidations for RodStringContent {
    fn get_validations(&self, field_name: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
        let mut validations = Vec::with_capacity(2);
        
        if let Some(length) = &self.length {
            validations.push(length.validate_string(field_name.clone()));
        }

        #[cfg(feature = "regex")]
        if let Some(format) = &self.format {
            let regex = match format {
                StringFormat::Regex(lit_str) => lit_str.value(),
                StringFormat::Email => String::from(regex_literals::EMAIL_REGEX),
                StringFormat::Url => String::from(regex_literals::URL_REGEX),
                StringFormat::Uuid => String::from(regex_literals::UUID_REGEX),
                StringFormat::Ipv4 => String::from(regex_literals::IPV4_REGEX),
                StringFormat::Ipv6 => String::from(regex_literals::IPV6_REGEX),
                StringFormat::DateTime => String::from(regex_literals::DATETIME_REGEX),
            };
            validations.push(quote! {
                if !regex::Regex::new(#regex).unwrap().is_match(&#field_name) {
                    let name = String::from(&#field_name);
                    return Err(RodValidateError::String(StringValidation::Format(name, #format)));
                }
            });
        }

        validations
    }
}

impl Parse for RodStringContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut length = None;
        #[cfg(feature = "regex")]
        let mut format = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                if ident == "length" {
                    check_already_used_attr!(length, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    length = Some(input.parse()?);
                } else if ident == "format" {
                    #[cfg(feature = "regex")]
                    {
                        check_already_used_attr!(format, ident.span());
                        input.parse::<syn::Token![:]>()?;
                        format = Some(input.parse()?);
                    }
                    #[cfg(not(feature = "regex"))]
                    {
                        abort!(ident.span(), "The `format` attribute is not available. Please enable the `regex` feature.");
                    }
                } else {
                    return Err(lookahead.error());
                }
            } else {
                return Err(lookahead.error());
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(RodStringContent { 
            length, 
            #[cfg(feature = "regex")]
            format 
        })
    }
}