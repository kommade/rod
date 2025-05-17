use quote::{format_ident, quote};
use syn::{parse::Parse, Ident, Index};

use crate::{RodAttr, RodAttrContent};

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations($field_access),
            )*
        }
    };
}

pub struct RodTupleContent {
    pub(crate) fields: Vec<RodAttr>,
}

impl Parse for RodTupleContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fields = input.parse_terminated(RodAttr::parse, syn::Token![,])?;
        let fields = fields.into_iter().collect();
        Ok(RodTupleContent { fields })
    }
}

impl RodTupleContent {
    pub(crate) fn get_validations(&self, field_name: &Ident) -> proc_macro2::TokenStream {
        self.fields.iter().enumerate().map(|(i, field)| {
            let i = Index::from(i);
            let subfield_name = format_ident!("{}_{}", field_name, i);
            let inner_validation = rod_content_match!(
                &field.content,
                &subfield_name,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom]
            );
            quote! {
                let #subfield_name = &#field_name.#i;
                #inner_validation
            }
        }).collect()
    }
}