use proc_macro_error::abort;
use quote::quote;
use quote::ToTokens;

use syn::{parse::Parse, LitStr};
use syn::Ident;


use super::{optional_braced, LengthOrSize};

#[cfg(feature = "regex")]
mod regex_literals {
    pub(crate) const EMAIL_REGEX: &str = r#"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])"#;
    pub(crate) const URL_REGEX: &str = r#"^[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&//=]*)$"#;
    pub(crate) const UUID_REGEX: &str = r#"(?i:^[0-9a-f]{8}-[0-9a-f]{4}-[0-5][0-9a-f]{3}-[089ab][0-9a-f]{3}-[0-9a-f]{12}$)"#;
    pub(crate) const IPV4_REGEX: &str = r#"^(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"#;
    pub(crate) const IPV6_REGEX: &str = r#"^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$"#;
    pub(crate) const DATETIME_REGEX: &str = r#"^(?:\d{4})-(?:\d{2})-(?:\d{2})T(?:\d{2}):(?:\d{2}):(?:\d{2}(?:\.\d*)?)(?:(?:-(?:\d{2}):(?:\d{2})|Z)?)$"#;
}

/// `StringFormat` is an enum that represents the format of a string field.
/// It includes variants for common formats such as email, URL, UUID, and IP addresses.
/// The `Regex` variant allows for custom regex patterns.
pub(crate) enum StringFormat {
    Regex(LitStr),
    Email,
    Url,
    Uuid,
    Ipv4,
    Ipv6,
    DateTime,
}

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
                _ => abort!(
                    ident.span(), "Unknown string format `{}`", ident;
                    help = "Valid string formats are: Email, Url, Uuid, Ipv4, Ipv6, DateTime, or a custom regex string literal.";
                ),
            }
        } else {
            abort!(input.span(), "Expected identifier or string literal for attribute `format`");
        }
    }
}

/// `RodStringContent` is a struct that represents the content of a string field in a Rod entity.
/// It is used to parse and validate string attributes in the `#[rod]` attribute macro.
/// This struct includes optional fields for length, format, starts_with, ends_with, and includes, 
/// which are used in validation checks.
/// # Attributes
/// - `length`: An optional attribute that specifies the length of the string.
/// - `format`: An optional attribute that specifies the format of the string, such as email, URL, UUID, or any custom regex. See [`StringFormat`][crate::types::string::StringFormat] enum. Note that this attribute requires the `regex` feature to be enabled.
/// - `starts_with`: An optional attribute that specifies the string must start with this value.
/// - `ends_with`: An optional attribute that specifies the string must end with this value.
/// - `includes`: An optional attribute that specifies the string must include this value.
/// # Usage
/// ```
/// use rod::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct MyEntity {
///    #[rod(
///        String {
///           length: 5..=20,
///           starts_with: "Hello",
///           ends_with: "World",
///           includes: "foo",
///        }
///    )]
///    my_field: String,
/// }
/// 
/// let entity = MyEntity {
///     my_field: "Hello foo World".to_string(),
/// };
/// 
/// assert!(entity.validate().is_ok());
/// ```
/// 
pub struct RodStringContent {
    length: Option<LengthOrSize>,
    format: Option<StringFormat>,
    starts_with: Option<LitStr>,
    ends_with: Option<LitStr>,
    includes: Option<LitStr>,
}

impl RodStringContent {
    pub(crate) fn get_validations(&self, field_name: &proc_macro2::Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let path = field_name.to_string();
        let length_opt = self.length.as_ref().map(|length| length.validate_string(field_name, wrap_return));
        #[cfg(feature = "regex")]
        let format_opt = self.format.as_ref().map(|format| {
            let regex = match format {
                StringFormat::Regex(lit_str) => lit_str.value(),
                StringFormat::Email => String::from(regex_literals::EMAIL_REGEX),
                StringFormat::Url => String::from(regex_literals::URL_REGEX),
                StringFormat::Uuid => String::from(regex_literals::UUID_REGEX),
                StringFormat::Ipv4 => String::from(regex_literals::IPV4_REGEX),
                StringFormat::Ipv6 => String::from(regex_literals::IPV6_REGEX),
                StringFormat::DateTime => String::from(regex_literals::DATETIME_REGEX),
            };
            let ret = wrap_return(quote!{ RodValidateError::String(StringValidation::Format(#path, name, #format)) });
            quote! {
                if !regex::Regex::new(#regex).unwrap().is_match(&#field_name) {
                    let name = String::from(&#field_name);
                    #ret;
                }
            }
        });
        #[cfg(not(feature = "regex"))]
        let format_opt: Option<proc_macro2::TokenStream> = None;
        let starts_with_opt = self.starts_with.as_ref().map(|starts_with| {
            let ret = wrap_return(quote!{ RodValidateError::String(StringValidation::StartsWith(#path, #field_name.clone().into(), #starts_with.into())) });
            quote! {
                if !#field_name.starts_with(#starts_with) {
                    #ret;
                }
            }
        });
        let ends_with_opt = self.ends_with.as_ref().map(|ends_with| {
            let ret = wrap_return(quote!{ RodValidateError::String(StringValidation::EndsWith(#path, #field_name.clone().into(), #ends_with.into())) });
            quote! {
                if !#field_name.ends_with(#ends_with) {
                    #ret;
                }
            }
        });
        let includes_opt = self.includes.as_ref().map(|includes| {
            let ret = wrap_return(quote!{ RodValidateError::String(StringValidation::Includes(#path, #field_name.clone().into(), #includes.into())) });
            quote! {
                if !#field_name.contains(#includes) {
                    #ret;
                }
            }
        });

        quote! {
            #length_opt
            #format_opt
            #starts_with_opt
            #ends_with_opt
            #includes_opt
        }
    }
}

impl Parse for RodStringContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(buffer) => buffer,
            None => return Ok(RodStringContent {
                length: None,
                format: None,
                starts_with: None,
                ends_with: None,
                includes: None,
            }),
        };

        let mut length = None;
        let mut format = None;
        let mut starts_with = None;
        let mut ends_with = None;
        let mut includes = None;

        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = inner.parse()?;
                if ident == "length" {
                    check_already_used_attr!(length, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    length = Some(inner.parse()?);
                } else if ident == "format" {
                    #[cfg(feature = "regex")]
                    {
                        check_already_used_attr!(format, ident.span());
                        inner.parse::<syn::Token![:]>()?;
                        format = Some(inner.parse()?);
                    }
                    #[cfg(not(feature = "regex"))]
                    {
                        abort!(ident.span(), "The `format` attribute is not available. Please enable the `regex` feature.");
                    }
                } else if ident == "includes" {
                    check_already_used_attr!(includes, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    includes = Some(inner.parse()?);
                } else if ident == "starts_with" {
                    check_already_used_attr!(starts_with, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    starts_with = Some(inner.parse()?);
                } else if ident == "ends_with" {
                    check_already_used_attr!(ends_with, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    ends_with = Some(inner.parse()?);
                } else {
                    abort!(
                        ident.span(),
                        "Unknown attribute `{}`", ident
                    );
                }
            } else {
                abort!(
                    inner.span(),
                    "Expected an identifier"
                );
            }

            _ = inner.parse::<syn::Token![,]>();
        }

        Ok(RodStringContent { 
            length, 
            format,
            starts_with,
            ends_with,
            includes,
        })
    }
}