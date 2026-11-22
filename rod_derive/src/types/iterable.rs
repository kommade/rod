use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitStr};
use quote::{format_ident, quote};

use crate::{RodAttr, RodAttrContent};

use super::{optional_braced, LengthOrSize};

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, $wrap_return:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations($field_access, $wrap_return),
            )*
        }
    };
    ($content:expr, $field_access:expr, $wrap_return:expr, $custom_error:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations_with_custom_error($field_access, $wrap_return, $custom_error),
            )*
        }
    };
}

pub struct RodIterableContent {
    pub(crate) item: Box<RodAttr>,
    pub(crate) length: Option<LengthOrSize>,
    pub(crate) distinct: bool,
    custom_item_error: Option<LitStr>,
    custom_length_error: Option<LitStr>,
}

impl Parse for RodIterableContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(inner) => inner,
            None => {
                abort!(
                    input.span(),
                    "Type Iterable must have an `item` attribute";
                    help = "Example: `#[rod(Iterable { item: String, length: 10 })]`"
                );
            }
        };
        let mut item = None;
        let mut length = None;
        let mut distinct = false;
        let mut custom_item_error: Option<LitStr> = None;
        let mut custom_length_error: Option<LitStr> = None;
        let mut message: Option<LitStr> = None;
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = inner.parse()?;
                if ident == "item" {
                    check_already_used_attr!(item, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    item = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_item_error = Some(msg);
                    }
                } else if ident == "length" || ident == "size" {
                    check_already_used_attr!(length, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    length = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_length_error = Some(msg);
                    }
                } else if ident == "distinct" {
                    // Boolean flag: bare `distinct` means true;
                    // `distinct = <bool>` also accepted.
                    if inner.peek(syn::Token![=]) || inner.peek(syn::Token![:]) {
                        inner.parse::<syn::Token![=]>().or_else(|_| inner.parse())?;
                        let flag: syn::LitBool = inner.parse()?;
                        distinct = flag.value;
                    } else {
                        distinct = true;
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

        if let Some(item) = item {
            Ok(RodIterableContent {
                item: Box::new(item),
                length,
                distinct,
                custom_item_error,
                custom_length_error,
            })
        } else {
            abort!(
                input.span(), "Type Iterable must have an `item` attribute";
                help = "Example: `#[rod(Iterable { item: String, length: 10 })]`"
            );
        }
    }
}

impl RodIterableContent {
    pub(crate) fn length(&self) -> Option<&LengthOrSize> {
        self.length.as_ref()
    }

    /// Generates the duplicate-item check when `distinct` was requested.
    /// Uses a `HashSet`, so item types must implement `Eq + Hash`.
    fn distinct_validation(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if !self.distinct {
            return quote! {};
        }
        // Derive-time path string: interpolating the field access instead
        // would call `to_string()` on the collection at runtime.
        let path = field_name.to_string();
        let ret = wrap_return(quote! {
            RodValidateError::Iterable(IterableValidation::Distinct(
                #path,
            ))
        });
        quote! {
            if {
                let mut seen = ::std::collections::HashSet::new();
                #field_name.iter().any(|item| !seen.insert(item))
            } {
                #ret;
            }
        }
    }

    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let inner_validation = if let Some(msg) = self.custom_item_error.as_ref() {
            rod_content_match!(
                &self.item.content,
                &format_ident!("item"),
                wrap_return,
                msg,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            )
        } else {
            rod_content_match!(
                &self.item.content,
                &format_ident!("item"),
                wrap_return,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            )
        };
        let length_opt = self.length.as_ref().map(|length| {
            if let Some(msg) = self.custom_length_error.as_ref() {
                length.validate_iterable_with_custom_error(field_name, wrap_return, msg)
            } else {
                length.validate_iterable(field_name, wrap_return)
            }
        });
        let distinct_check = self.distinct_validation(field_name, wrap_return);
        quote! {
            #length_opt
            #distinct_check
            for item in #field_name.into_iter() {
                #inner_validation
            }
        }
    }
    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        let inner_validation_with_custom_error = if let Some(msg) = self.custom_item_error.as_ref() {
            rod_content_match!(
                &self.item.content,
                &format_ident!("item"),
                wrap_return,
                msg,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            )
        } else {
            rod_content_match!(
                &self.item.content,
                &format_ident!("item"),
                wrap_return,
                custom_error,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            )
        };
        let length_opt = self.length.as_ref().map(|length| {
            if let Some(msg) = self.custom_length_error.as_ref() {
                length.validate_iterable_with_custom_error(field_name, wrap_return, msg)
            } else {
                length.validate_iterable_with_custom_error(field_name, wrap_return, custom_error)
            }
        });
        let distinct_check = self.distinct_validation(field_name, wrap_return);
        quote! {
            #length_opt
            #distinct_check
            for item in #field_name.into_iter() {
                #inner_validation_with_custom_error
            }
        }
    
    }
}