extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{ToTokens, quote};
use statics::*;
use syn::parse::{Parse, ParseStream};
use syn::{
    Data, DeriveInput, Expr, ExprRange, Fields, Ident, LitStr, Result as SynResult, Token, Type,
    braced, parse_macro_input,
};

mod statics;

enum RodAttrType {
    String,
    Integer,
}
macro_rules! assert_type {
    ($ty:expr, $expected:expr) => {
        match $expected {
            RodAttrType::String => {
                if !STRING_VARIANTS.contains(&$ty.to_string().as_str()) {
                    abort!(
                        proc_macro::Span::call_site(), "Expected {} to be a string type", $ty;
                        help = "Check if the type is a string type: one of String, str, OsString, OsStr, PathBuf, Path, or Cow";
                        note = "If you are using a custom type, ensure it implements RodValidate"
                    );
                }
            }
            RodAttrType::Integer => {
                if !INTEGER_VARIANTS.contains(&$ty.to_string().as_str()) {
                    abort!(
                        proc_macro::Span::call_site(), "Expected {} to be an integer type", $ty;
                        help = "Check if the type is an integer type: one of i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, or usize";
                        note = "If you are using a custom type, ensure it implements RodValidate"
                    );
                }
            }
        }
    };
}

struct RodNone;

impl Parse for RodNone {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let skip: Ident = input.parse()?;
        if skip != "none" {
            return Err(input.error("Expected `none`"));
        }
        Ok(RodNone)
    }
}

struct RodAttr {
    ty: Type,
    content: RodAttrContent,
}

struct RodAttrContent {
    // String attributes
    length: Option<Expr>,
    format: Option<LitStr>,

    // integer attributes
    range: Option<ExprRange>,
}

impl Parse for RodAttr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ty: Type = input.parse()?;
        let type_ident = if let syn::Type::Path(type_path) = &ty {
            if let Some(ident) = type_path.path.segments.last().map(|s| &s.ident) {
                ident.to_string()
            } else {
                return Err(input.error("Expected a type identifier"));
            }
        } else {
            return Err(input.error("Expected a path type"));
        };
        let content;
        braced!(content in input);

        let mut length = None;
        let mut format = None;
        let mut range = None;

        while !content.is_empty() {
            let key: Ident = content.parse().or(Err(
                content.error(format!("Expected attribute key, found `{}`", content))
            ))?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "length" => {
                    assert_type!(type_ident, RodAttrType::String);
                    length = Some(content.parse()?);
                }
                "format" => {
                    assert_type!(type_ident, RodAttrType::String);
                    format = Some(content.parse()?);
                }
                "range" => {
                    assert_type!(type_ident, RodAttrType::Integer);
                    range = Some(content.parse::<ExprRange>()?);
                }
                other => {
                    return Err(content.error(format!("Unknown attribute key {}", other)));
                }
            }
            let _ = content.parse::<Token![,]>();
        }

        Ok(RodAttr {
            ty,
            content: RodAttrContent {
                length,
                format,
                range,
            },
        })
    }
}
/// Derives the `RodValidate` trait for a struct.
///
/// Implements validation logic for struct fields annotated with `#[rod(...)]`.
/// Fields without the attribute are required to implement `RodValidate`.
/// String and integer constraints are supported via the attribute.
/// 
/// # Examples
/// 
/// ```
/// use rod_derive::RodValidate;
///
/// #[derive(RodValidate)]
/// struct User {
///     #[rod(
///         String {
///             length: 3..=12, // Length between 3 and 12 characters
///             format: "^[a-zA-Z0-9_]+$", // Alphanumeric and underscores only
///         }
///     )]
///     username: String,
///     #[rod(
///         Integer {
///             range: 18..=99, // Age between 18 and 99
///         }
///     )]
///     age: u8,
/// }
/// ```
/// 
/// 
/// # Invalid Examples
/// 
/// This example demonstrates a struct that does not implement `RodValidate`.
/// 
/// ```compile_fail
/// use rod_derive::RodValidate;
///
/// struct DoesNotImplementRodValidate {
///     field: String,
/// }
///
/// #[derive(RodValidate)] // This will fail to compile
/// struct Test {
///     #[rod(
///         String {
///             length: 5..=10,
///             format: "^[a-zA-Z]+$",
///         }
///     )]
///     field1: String,
///     field2: DoesNotImplementRodValidate, // This field does not implement RodValidate
/// }
/// ```
#[proc_macro_error]
#[proc_macro_derive(RodValidate, attributes(rod))]
pub fn derive_rod_validate(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let mut validations = Vec::new();

    if let Data::Struct(data_struct) = &ast.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                let field_name = &field.ident;
                let field_type = if let syn::Type::Path(type_path) = &field.ty {
                    if let Some(ident) = type_path.path.segments.last().map(|s| &s.ident) {
                        ident
                    } else {
                        panic!("Expected a type identifier");
                    }
                } else {
                    panic!("Expected a path type");
                };
                let field_type_string = field_type.to_string();
                if field.attrs.is_empty() {
                    validations.push(quote! {
                        fn assert_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                            value.validate()
                        }
                        assert_rod_validate(&self.#field_name)?;
                    });
                } else {
                    for attr in &field.attrs {
                        if attr.path().is_ident("rod") {
                            // Check for #[rod(skip)]
                            if attr.parse_args::<RodNone>().is_ok() {
                                // Do not generate validation for this field
                                continue;
                            }
                            match attr.parse_args::<RodAttr>() {
                                Ok(rod_attr) => {
                                    // Use syn::Type::Path to match type name robustly
                                    if let syn::Type::Path(type_path) = &rod_attr.ty {
                                        if let Some(ident) = type_path
                                            .path
                                            .segments
                                            .last()
                                            .map(|s| s.ident.to_string())
                                        {
                                            if STRING_VARIANTS.contains(&ident.as_str()) {
                                                assert_type!(field_type_string, RodAttrType::String);
                                                if let Some(length_expr) =
                                                    rod_attr.content.length.as_ref()
                                                {
                                                    validations.push(quote! {
                                                        if !(#length_expr).contains(&self.#field_name.len()) {
                                                            return Err(RodValidateError::InvalidLength);
                                                        }
                                                    });
                                                }
                                                if let Some(format_str) =
                                                    rod_attr.content.format.as_ref()
                                                {
                                                    validations.push(quote! {
                                                        let re = regex::Regex::new(#format_str).unwrap();
                                                        if !re.is_match(&self.#field_name) {
                                                            return Err(RodValidateError::InvalidFormat);
                                                        }
                                                    });
                                                }
                                            } else if INTEGER_VARIANTS.contains(&ident.as_str()) {
                                                assert_type!(field_type_string, RodAttrType::Integer);
                                                if let Some(range_expr) =
                                                    rod_attr.content.range.as_ref()
                                                {
                                                    validations.push(quote! {
                                                        if !(#range_expr).contains(&self.#field_name) {
                                                            return Err(RodValidateError::OutOfRange);
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    panic!(
                                        "Failed to parse attribute for field `{}`: {}",
                                        field_name.as_ref().unwrap(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // ...existing code...

    quote! {
        impl RodValidate for #name {
            fn validate(&self) -> Result<(), RodValidateError> {
                #(#validations)*
                Ok(())
            }
        }
    }
    .into()
}
// ...existing code...
