use proc_macro_error::abort;
use syn::{parse::Parse, Ident, LitStr};
use quote::{format_ident, quote};

use crate::{RodAttr, RodAttrContent};

use super::{optional_braced, user_defined_error};

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

/// `RodOptionContent` is a struct that represents the content of an option field in a Rod entity.
/// It is used to parse and validate option attributes in the `#[rod]` attribute macro.
/// This struct includes a single field `inner`, which stores the content of the option attribute, that could be any other validation type, including `Option`.
/// # Attributes
/// None, as `inner` is not meant to be set directly. If you want to validate the content of an option, you should place the validation type inside the `Option` attribute.
/// if you want to validate that the option is `None`, you can use `Option {}`.
/// # Usage
/// ```
/// use rod::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct MyEntity {
///     #[rod(
///         Option {
///             String {
///                 length: 5,
///             }
///         }
///     )]
///     my_field: Option<String>,
///     #[rod(Option {})]
///     none_field: Option<String>,
/// }
/// 
/// let entity = MyEntity {
///    my_field: Some("12345".to_string()),
///    none_field: None,
/// };
/// assert!(entity.validate().is_ok());
/// ```
pub struct RodOptionContent {
    pub(crate) inner: Option<Box<RodAttr>>,
    custom_none_error: Option<LitStr>,
}

impl Parse for RodOptionContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(inner) => inner,
            None => {
                return Ok(RodOptionContent { inner: None, custom_none_error: None });
            }
        };
        if inner.is_empty() {
            Ok(RodOptionContent { inner: None, custom_none_error: None })
        } else {
            let mut rod_attr: Option<RodAttr> = None;
            let mut message: Option<LitStr> = None;
            while !inner.is_empty() {
                let lookahead = inner.lookahead1();
                if lookahead.peek(syn::Token![?]) {
                    let _q: syn::Token![?] = inner.parse()?;
                    let msg: LitStr = inner.parse()?;
                    message = Some(msg);
                } else {
                    if rod_attr.is_some() {
                        abort!(inner.span(), "Option attribute can only contain a single inner validation");
                    }
                    rod_attr = Some(inner.parse()?);
                }
                _ = inner.parse::<syn::Token![,]>();
            }
            Ok(RodOptionContent {
                inner: rod_attr.map(Box::new),
                custom_none_error: message,
            })
        }
    }
}

impl RodOptionContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let path = field_name.to_string();
        if self.inner.is_none() {
            let ret = if let Some(msg) = self.custom_none_error.as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Option(OptionValidation::Some(
                        #path,
                        format!("{:?}", #field_name)
                    ))
                })
            };
            quote! {
                if #field_name.is_some() {
                    #ret;
                }
            }
        } else {
            let inner_validation = rod_content_match!(
                &self.inner.as_ref().unwrap().content,
                &format_ident!("opt"),
                wrap_return,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            );
            let ty = self.inner.as_ref().unwrap().ty.to_string();
            let ret = if let Some(msg) = self.custom_none_error.as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                wrap_return(quote! {
                    RodValidateError::Option(OptionValidation::None(#path, #ty))
                })
            };
            quote! {
                match &#field_name {
                    Some(opt) => {
                        #inner_validation
                    }
                    None => {
                        #ret;
                    }
                }
            }
        }
    }
    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        if self.inner.is_none() {
            let ret = if let Some(msg) = self.custom_none_error.as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                user_defined_error(wrap_return, custom_error)
            };
            quote! {
                if #field_name.is_some() {
                    #ret;
                }
            }
        } else {
            let inner_validation = rod_content_match!(
                &self.inner.as_ref().unwrap().content,
                &format_ident!("opt"),
                wrap_return,
                custom_error,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
            );
            let ret = if let Some(msg) = self.custom_none_error.as_ref() {
                user_defined_error(wrap_return, msg)
            } else {
                user_defined_error(wrap_return, custom_error)
            };
            quote! {
                match &#field_name {
                    Some(opt) => {
                        #inner_validation
                    }
                    None => {
                        #ret;
                    }
                }
            }
        }
    }
}