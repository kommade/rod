use proc_macro_error::abort;
use syn::{parse::Parse, Ident, TypePath};
use quote::{format_ident, quote};

use crate::{RodAttr, RodAttrContent, TypeEnum};

use super::LengthOrSize;

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations($field_access),
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
        let mut item = None;
        let mut length = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                if ident == "item" {
                    check_already_used_attr!(item, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    item = Some(input.parse()?);
                } else if ident == "length" || ident == "size" {
                    check_already_used_attr!(length, ident.span());
                    input.parse::<syn::Token![:]>()?;
                    length = Some(input.parse()?);
                } else {
                    abort!(
                        ident.span(),
                        "Unknown attribute `{}`", ident
                    );
                }
                _ = input.parse::<syn::Token![,]>();
            } else {
                abort!(
                    input.span(),
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
            abort!(input.span(), "Type Iterable must have an `item` attribute");
        }
        
    }
}

impl RodIterableContent {
    pub(crate) fn get_validations(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        let inner_validation = rod_content_match!(
            &self.item.content,
            &format_ident!("item"),
            [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
        );
        let length_opt = self.length.as_ref().map(|length| length.validate_iterable(field_name));
        quote! {
            #length_opt
            for item in #field_name.into_iter() {
                #inner_validation
            }
        }
    }
}
