use proc_macro_error::{abort, emit_warning};
use syn::{parse::Parse, Ident, LitStr};
use quote::quote;

use crate::{RodAttr, RodAttrContent};

use super::{optional_braced, LengthOrSize};

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, $wrap_return:expr, $delegate:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations($field_access, $wrap_return, $delegate),
            )*
        }
    };
    ($content:expr, $field_access:expr, $wrap_return:expr, $delegate:expr, $custom_error:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(content) => content.get_validations_with_custom_error($field_access, $wrap_return, $delegate, $custom_error),
            )*
        }
    };
}

/// `RodMapContent` validates key-value collections (`HashMap`/`BTreeMap`):
/// every key is checked with the `key` rule, every value with the `value`
/// rule, and the whole map with an optional `length` (entry count).
///
/// Entries are validated explicitly - never silently: omitting BOTH `key`
/// and `value` is a compile error, and `key: Skip` / `value: Skip` are the
/// explicit escape hatches. Per-entry errors keep their entry path:
/// key errors are nested as `key`, value errors as `value` (so delegation
/// into a `RodValidate` value type renders `field.value: <inner error>`),
/// exactly like the `item` binding of `Iterable`.
///
/// The `length` of a map is its entry count.
///
/// # Usage
///
/// ```
/// use std::collections::HashMap;
/// use rod_validation::prelude::*;
///
/// #[derive(RodValidate)]
/// struct Cart {
///     #[rod(Map {
///         key: String { length: 1..=30 },
///         value: u32 { size: 1..=999 },
///         length: ..=50,
///     })]
///     quantities: HashMap<String, u32>,
/// }
/// ```
///
/// Delegating values to their own `RodValidate` impl (per-entry `value`
/// path):
///
/// ```
/// use std::collections::HashMap;
/// use rod_validation::prelude::*;
///
/// #[derive(RodValidate)]
/// struct Line { number: u32 }
///
/// #[derive(RodValidate)]
/// struct Invoice {
///     #[rod(HashMap { value: Line })]
///     lines: HashMap<String, Line>,
/// }
/// ```
///
/// A key-value rule on a sequence field is a compile error:
///
/// ```compile_fail
/// use rod_validation::prelude::*;
///
/// #[derive(RodValidate)]
/// struct Cart {
///     #[rod(Map { key: Skip, value: String })]
///     items: Vec<String>,
/// }
/// ```
///
/// A map with neither a `key` nor a `value` rule is a compile error:
///
/// ```compile_fail
/// use std::collections::HashMap;
/// use rod_validation::prelude::*;
///
/// #[derive(RodValidate)]
/// struct Cart {
///     #[rod(Map { length: 3 })]
///     items: HashMap<String, u8>,
/// }
/// ```
pub struct RodMapContent {
    /// Rule for every entry key. `Skip` leaves keys unchecked;
    /// `Custom` (or a named type) delegates to the key type's own impl,
    /// with per-entry errors nested under the `key` path.
    pub(crate) key: Option<Box<RodAttr>>,
    /// Rule for every entry value - same escape hatches as `key`.
    pub(crate) value: Option<Box<RodAttr>>,
    /// Entry-count check on the whole map.
    length: Option<LengthOrSize>,
    custom_key_error: Option<LitStr>,
    custom_value_error: Option<LitStr>,
    custom_length_error: Option<LitStr>,
}

impl Parse for RodMapContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(inner) => inner,
            None => {
                abort!(
                    input.span(),
                    "Type Map needs at least a `key` or a `value` rule";
                    help = "Example: `#[rod(Map { key: String { length: 1..=10 }, value: u32 })]`"
                );
            }
        };
        let mut key: Option<Box<RodAttr>> = None;
        let mut value: Option<Box<RodAttr>> = None;
        let mut length: Option<LengthOrSize> = None;
        let mut custom_key_error: Option<LitStr> = None;
        let mut custom_value_error: Option<LitStr> = None;
        let mut custom_length_error: Option<LitStr> = None;
        let mut message: Option<LitStr> = None;
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = inner.parse()?;
                if ident == "key" {
                    if key.is_some() {
                        emit_warning!(
                            ident.span(),
                            "The attribute `key` is used multiple times. The last time it was specified will take precedence."
                        );
                    }
                    inner.parse::<syn::Token![:]>()?;
                    key = Some(Box::new(inner.parse()?));
                    if let Some(msg) = message.take() {
                        custom_key_error = Some(msg);
                    }
                } else if ident == "value" {
                    if value.is_some() {
                        emit_warning!(
                            ident.span(),
                            "The attribute `value` is used multiple times. The last time it was specified will take precedence."
                        );
                    }
                    inner.parse::<syn::Token![:]>()?;
                    value = Some(Box::new(inner.parse()?));
                    if let Some(msg) = message.take() {
                        custom_value_error = Some(msg);
                    }
                } else if ident == "length" || ident == "size" {
                    if length.is_some() {
                        emit_warning!(
                            ident.span(),
                            "The attribute `length` is used multiple times. The last time it was specified will take precedence."
                        );
                    }
                    inner.parse::<syn::Token![:]>()?;
                    length = Some(inner.parse()?);
                    if let Some(msg) = message.take() {
                        custom_length_error = Some(msg);
                    }
                } else if ident == "distinct" {
                    abort!(
                        ident.span(),
                        "`distinct` is not valid on a `Map`";
                        help = "Map keys are unique by construction - there is nothing to check. Remove `distinct`."
                    );
                } else {
                    abort!(
                        ident.span(),
                        "Unknown attribute `{}`", ident;
                        help = "Valid Map keys are: `key`, `value` and `length`."
                    );
                }
                _ = inner.parse::<syn::Token![,]>();
            } else if lookahead.peek(syn::Token![?]) {
                // `? "message"` customizes the error of the NEXT rule below.
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

        if key.is_none() && value.is_none() {
            abort!(
                input.span(),
                "Type Map needs a `key` or a `value` rule";
                help = "Entries are validated explicitly, never silently - use `key: Skip` / `value: Skip` to opt out of one side. Example: `Map {{ key: Skip, value: u32 {{ size: 1.. }} }}`"
            );
        }

        Ok(RodMapContent {
            key,
            value,
            length,
            custom_key_error,
            custom_value_error,
            custom_length_error,
        })
    }
}

impl RodMapContent {
    pub(crate) fn length(&self) -> Option<&LengthOrSize> {
        self.length.as_ref()
    }

    /// Generates the validation for one side (key or value) of every
    /// entry. `Skip` as the side's rule leaves it entirely unchecked -
    /// inner skipping is always explicit, never a default.
    fn side_validation(
        side: Option<&Box<RodAttr>>,
        access: &Ident,
        wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        custom_error: Option<&LitStr>,
    ) -> proc_macro2::TokenStream {
        let Some(side) = side else {
            return quote! {};
        };
        // `Skip` (e.g. `Map { key: Skip, value: String }`): this side of
        // every entry is left unchecked, explicitly.
        if matches!(side.content, RodAttrContent::Skip(_)) {
            return quote! {};
        }
        match custom_error {
            Some(msg) => rod_content_match!(
                &side.content,
                &access,
                wrap_return,
                delegate,
                msg,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable, Map]
            ),
            None => rod_content_match!(
                &side.content,
                &access,
                wrap_return,
                delegate,
                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable, Map]
            ),
        }
    }

    fn length_validation(
        &self,
        field_name: &Ident,
        wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        custom_error: Option<&LitStr>,
    ) -> proc_macro2::TokenStream {
        let Some(length) = self.length.as_ref() else {
            return quote! {};
        };
        let error = self.custom_length_error.as_ref().or(custom_error);
        match error {
            Some(msg) => length.validate_iterable_with_custom_error(field_name, wrap_return, msg),
            None => length.validate_iterable(field_name, wrap_return),
        }
    }
}

impl RodMapContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let key_ident = quote::format_ident!("key");
        let value_ident = quote::format_ident!("value");
        let key_validation = Self::side_validation(self.key.as_ref(), &key_ident, wrap_return, delegate, None);
        let value_validation = Self::side_validation(self.value.as_ref(), &value_ident, wrap_return, delegate, None);
        let length_check = self.length_validation(field_name, wrap_return, None);
        quote! {
            #length_check
            for (key, value) in #field_name.into_iter() {
                #key_validation
                #value_validation
            }
        }
    }
    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        let key_ident = quote::format_ident!("key");
        let value_ident = quote::format_ident!("value");
        let key_validation = Self::side_validation(self.key.as_ref(), &key_ident, wrap_return, delegate, self.custom_key_error.as_ref());
        let value_validation = Self::side_validation(self.value.as_ref(), &value_ident, wrap_return, delegate, self.custom_value_error.as_ref());
        let length_check = self.length_validation(field_name, wrap_return, Some(custom_error));
        quote! {
            #length_check
            for (key, value) in #field_name.into_iter() {
                #key_validation
                #value_validation
            }
        }
    }
}
