use proc_macro_error::abort;
use syn::{parse::Parse, Ident, TypePath};
use quote::{format_ident, quote};

use crate::{RodAttr, RodAttrContent, TypeEnum};

use super::{optional_braced, LengthOrSize};

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, $wrap_return:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations($field_access, $wrap_return),
            )*
        }
    };
}

pub struct RodIterableContent {
    pub(crate) item: Box<RodAttr>,
    pub(crate) length: Option<LengthOrSize>,
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
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = inner.parse()?;
                if ident == "item" {
                    check_already_used_attr!(item, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    item = Some(inner.parse()?);
                } else if ident == "length" || ident == "size" {
                    check_already_used_attr!(length, ident.span());
                    inner.parse::<syn::Token![:]>()?;
                    length = Some(inner.parse()?);
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

        if let Some(item) = item {
            Ok(RodIterableContent {
                item: Box::new(item),
                length,
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
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let inner_validation = rod_content_match!(
            &self.item.content,
            &format_ident!("item"),
            wrap_return,
            [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
        );
        let length_opt = self.length.as_ref().map(|length| length.validate_iterable(field_name, wrap_return));
        quote! {
            #length_opt
            for item in #field_name.into_iter() {
                #inner_validation
            }
        }
    }
}
