use proc_macro_error::abort;
use syn::{parse::Parse, spanned::Spanned, Ident, LitStr, PatLit};
use quote::quote;

use super::{optional_braced, user_defined_error};


/// `RodLiteralContent` is a struct that represents the content of a literal field in a Rod entity.
/// It is used to parse and validate literal attributes in the `#[rod]` attribute macro.
/// This struct includes either a single expectation, which checks that the
/// field equals one exact literal, or a list `any`, which checks that the
/// field equals **any** of the given literals (an enum-like value set).
/// # Attributes
/// - `is`: The exact expected literal (`is: 42`). Canonical form.
/// - `value`: Deprecated alias of `is`. Still works, but emits a
///   deprecation warning at compile time.
/// - `any`: A comma-separated list of accepted literals
///   (`any: ["admin", "mod"]` or `any: (1, 2, 3)`).
/// Exactly one of `is`/`value` or `any` must be given.
/// # Usage
/// ```
/// use rod_validation::prelude::*;
/// 
/// #[derive(RodValidate)]
/// struct MyEntity {
///   #[rod(
///        literal {
///           is: 42,
///        }
///   )]
///   my_field: i32,
///   #[rod(
///        Literal {
///           any: ["admin", "mod"],
///        }
///   )]
///   role: String,
/// }
/// 
/// let entity = MyEntity { my_field: 42, role: "admin".to_string() };
/// assert!(entity.validate().is_ok());
/// ```
pub struct RodLiteralContent {
    /// Single expected literal: canonical `is` or deprecated `value`.
    value: Option<PatLit>,
    /// Accepted literals for `any: [...]` / `any: (...)`.
    any: Vec<PatLit>,
    custom_error: Option<LitStr>,
}

impl RodLiteralContent {
    /// The single expected literal, if the `is`/`value` form was used.
    pub(crate) fn value(&self) -> Option<&PatLit> {
        self.value.as_ref()
    }

    /// The accepted literal set, if the `any` form was used.
    pub(crate) fn any(&self) -> &[PatLit] {
        &self.any
    }
}

impl Parse for RodLiteralContent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let opt = optional_braced(input)?;
        let inner = match opt {
            Some(buffer) => buffer,
            None => {
                abort!(
                    input.span(),
                    "Must specify a literal value using `is: <literal>` or `any: [literals]` inside the `literal` attribute.";
                    help = "Example: `#[rod(Literal { is: 42 })]` or `#[rod(Literal { any: [\"a\", \"b\"] })]`"
                )
            }
        };
    let mut value: Option<PatLit> = None;
    let mut any: Option<Vec<PatLit>> = None;
    let mut message: Option<LitStr> = None;
    let mut custom_error: Option<LitStr> = None;
        while !inner.is_empty() {
            let lookahead = inner.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: syn::Ident = inner.parse()?;
                let ident_str = ident.to_string();
                match ident_str.as_str() {
                    "is" => {
                        // `is: <literal>`: canonical single-value form.
                        check_already_used_attr!(value, ident.span());
                        inner.parse::<syn::Token![:]>()?;
                        value = Some(inner.parse()?);
                        if let Some(msg) = message.take() {
                            custom_error = Some(msg);
                        }
                    }
                    "value" => {
                        // Deprecated alias of `is`; still works. Emitted
                        // eagerly so no state has to be carried to codegen;
                        // worded like rustc's own deprecation diagnostics.
                        proc_macro_error::emit_warning!(
                            ident.span(),
                            "use of deprecated item `Literal::value`: use `Literal::is` instead"
                        );
                        check_already_used_attr!(value, ident.span());
                        inner.parse::<syn::Token![:]>()?;
                        value = Some(inner.parse()?);
                        if let Some(msg) = message.take() {
                            custom_error = Some(msg);
                        }
                    }
                    "any" => {
                        check_already_used_attr!(any, ident.span());
                        inner.parse::<syn::Token![:]>()?;
                        // Accept both bracket (...) and paren (...) list
                        // forms; every element must be a literal.
                        let items: Vec<PatLit> = parse_pat_lit_list(&inner)?;
                        if items.is_empty() {
                            abort!(
                                inner.span(),
                                "`any` must not be empty";
                                help = "List the accepted literals, e.g. `any: [\"admin\", \"mod\"]`."
                            );
                        }
                        any = Some(items);
                        if let Some(msg) = message.take() {
                            custom_error = Some(msg);
                        }
                    }
                    _ => {
                        abort!(
                            ident.span(),
                            "Unknown attribute `{}`", ident;
                            help = "Valid attributes are `is`, `any` (and deprecated `value`)."
                        );
                    }
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
        match (value, any) {
            (Some(_), Some(_)) => {
                abort!(
                    input.span(),
                    "`Literal` accepts one expectation (`is`/`value`) or `any`, not both";
                    help = "Use `any` when more than one literal is acceptable."
                )
            }
            (None, None) => {
                abort!(
                    input.span(),
                    "Must specify a literal expectation using `is: <literal>` or `any: [literals]` inside the `literal` attribute.";
                    help = "Example: `#[rod(Literal { is: 42 })]` or `#[rod(Literal { any: [\"a\", \"b\"] })]`"
                )
            }
            (value, any) => {
                let custom_error = custom_error.or(message);
                Ok(RodLiteralContent {
                    value,
                    any: any.unwrap_or_default(),
                    custom_error,
                })
            }
        }
    }
}

/// Comma-separated literal list consumed from `inner`. Accepts a bracketed
/// or parenthesized group (`["a", "b"]` / `("a", "b")`); every element must
/// be a literal.
fn parse_pat_lit_list(inner: syn::parse::ParseStream) -> syn::Result<Vec<PatLit>> {
    let look = inner.lookahead1();
    if look.peek(syn::token::Bracket) {
        let arr: syn::ExprArray = inner.parse()?;
        expr_elems(arr.elems.iter())
    } else if look.peek(syn::token::Paren) {
        let tuple: syn::ExprTuple = inner.parse()?;
        expr_elems(tuple.elems.iter())
    } else {
        Err(look.error())
    }
}

/// Converts literal expression elements to `PatLit`s, rejecting anything else.
fn expr_elems<'a, I>(items: I) -> syn::Result<Vec<PatLit>>
where
    I: Iterator<Item = &'a syn::Expr>,
{
    items
        .map(|item| match item {
            syn::Expr::Lit(lit_expr) => Ok(PatLit {
                attrs: Vec::new(),
                lit: lit_expr.lit.clone(),
            }),
            other => Err(syn::Error::new(
                other.span(),
                "Expected a literal value (string, number, boolean or char)",
            )),
        })
        .collect()
}

impl RodLiteralContent {
    pub(crate) fn get_validations(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        // The error variants carry the field path as &'static str.
        let field_name_str = quote! { stringify!(#field_name) };
        match self.expectation() {
            LiteralExpectation::Single(value) => {
                let ret = if let Some(msg) = self.custom_error.as_ref() {
                    user_defined_error(wrap_return, msg)
                } else {
                    wrap_return(quote! {
                        RodValidateError::Literal(LiteralValidation::Value(#field_name_str, #field_name.clone().to_string(), format!("to be {}", #value)))
                    })
                };
                quote! {
                    if #field_name.clone() != #value {
                        #ret;
                    }
                }
            }
            LiteralExpectation::Any(values) => {
                // Match against the first accepted literal, then the rest.
                let ret = if let Some(msg) = self.custom_error.as_ref() {
                    user_defined_error(wrap_return, msg)
                } else {
                    wrap_return(quote! {
                        RodValidateError::Literal(LiteralValidation::Value(#field_name_str, #field_name.clone().to_string(), format!("to be one of {:?}", vec![#(#values.to_string()),*])))
                    })
                };
                quote! {
                    if !(#( #field_name.clone() == #values )||*) {
                        #ret;
                    }
                }
            }
        }
    }
    pub(crate) fn get_validations_with_custom_error(&self, field_name: &Ident, wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream, custom_error: &LitStr) -> proc_macro2::TokenStream {
        let _ = field_name;
        match self.expectation() {
            LiteralExpectation::Single(value) => {
                let ret = if let Some(msg) = self.custom_error.as_ref() {
                    user_defined_error(wrap_return, msg)
                } else {
                    user_defined_error(wrap_return, custom_error)
                };
                quote! {
                    if #field_name.clone() != #value {
                        #ret;
                    }
                }
            }
            LiteralExpectation::Any(values) => {
                let ret = if let Some(msg) = self.custom_error.as_ref() {
                    user_defined_error(wrap_return, msg)
                } else {
                    user_defined_error(wrap_return, custom_error)
                };
                quote! {
                    if !(#( #field_name.clone() == #values )||*) {
                        #ret;
                    }
                }
            }
        }
    }
}

/// Which form the literal rule was written in.
enum LiteralExpectation<'a> {
    Single(&'a PatLit),
    Any(Vec<&'a PatLit>),
}

impl RodLiteralContent {
    fn expectation(&self) -> LiteralExpectation<'_> {
        if let Some(value) = &self.value {
            LiteralExpectation::Single(value)
        } else {
            LiteralExpectation::Any(self.any.iter().collect())
        }
    }
}