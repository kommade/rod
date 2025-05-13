extern crate proc_macro;
use std::fmt::Display;

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    braced, parse_macro_input, Data, DeriveInput, Fields, Ident, Result as SynResult, Type
};
use types::{RodIntegerContent, RodStringContent};

mod types;

fn get_type_ident(ty: &Type) -> Option<&Ident> {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| &s.ident)
    } else {
        None
    }
}

macro_rules! assert_type {
    ($name:expr, $ty:expr, $expected:expr) => {
        let actual_type: RodAttrType = $ty.into();
        if actual_type != $expected {
            abort!(
                $name.span(), "Expected `{}` to be a {} type, but found {}", 
                $name.unwrap(), $expected, actual_type; 
                help = "The type of the field must match the expected type in the attribute.";
            );
        }
    };
}

#[derive(Debug, PartialEq)]
enum RodAttrType {
    String(Ident),
    Integer(Ident),
}

impl Display for RodAttrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RodAttrType::String(ident) => write!(f, "String({})", ident),
            RodAttrType::Integer(ident) => write!(f, "Integer({})", ident),
        }
    }
}

impl From<Type> for RodAttrType {
    fn from(ty: Type) -> RodAttrType {
        let type_ident = get_type_ident(&ty).unwrap_or_else(|| {
            abort!(
                ty.span(), "Expected a type path, but found: {:?}", ty
            );
        });
        match type_ident.to_string().as_str() {
            "String" | "str" | "OsString" | "OsStr" | "PathBuf" | "Path" | "Cow" => RodAttrType::String(type_ident.clone()),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
            | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => RodAttrType::Integer(type_ident.clone()),
            _ => abort!(
                ty.span(), "Unsupported type: {}", type_ident
            ),
        }
    }
}

impl From<&Type> for RodAttrType {
    fn from(ty: &Type) -> RodAttrType {
        let type_ident = get_type_ident(ty).unwrap_or_else(|| {
            abort!(
                ty.span(), "Expected a type path, but found: {:?}", ty
            );
        });
        match type_ident.to_string().as_str() {
            "String" | "str" | "OsString" | "OsStr" | "PathBuf" | "Path" | "Cow" => RodAttrType::String(type_ident.clone()),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
            | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => RodAttrType::Integer(type_ident.clone()),
            _ => abort!(
                ty.span(), "Unsupported type: {}", type_ident
            ),
        }
    }
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

pub(crate) trait GetValidations {
    fn get_validations(&self, field_name: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream>;
}

struct RodAttr {
    ty: RodAttrType,
    content: RodAttrContent,
}

enum RodAttrContent {
    String(RodStringContent),
    Integer(RodIntegerContent),
}

impl Parse for RodAttr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ty: Type = input.parse()?;
        let rod_type: RodAttrType = ty.into();
        let inner;
        braced!(inner in input);
        let content = match rod_type {
            RodAttrType::String(_) => {
                let string_content: RodStringContent = inner.parse()?;
                RodAttrContent::String(string_content)
            }
            RodAttrType::Integer(_) => {
                let integer_content: RodIntegerContent = inner.parse()?;
                RodAttrContent::Integer(integer_content)
            }
        };
        Ok(RodAttr { ty: rod_type, content })
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
                            if attr.parse_args::<RodNone>().is_ok() {
                                continue;
                            }
                            match attr.parse_args::<RodAttr>() {
                                Ok(rod_attr) => {
                                    let rod_attr_type = rod_attr.ty;
                                    assert_type!(field_name.as_ref(), &field.ty, rod_attr_type);
                                    let validations_for_field = match rod_attr.content {
                                        RodAttrContent::String(ref content) => content.get_validations(
                                            quote! { self.#field_name }
                                        ),
                                        RodAttrContent::Integer(ref content) => content.get_validations(
                                            quote! { self.#field_name }
                                        ),
                                    };
                                    validations.extend(validations_for_field);
                                }
                                Err(e) => {
                                    abort!(
                                        attr.span(), "Failed to parse attribute: {}", e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if let Data::Enum(data_enum) = &ast.data {
        for variant in &data_enum.variants {
            let variant_ident = &variant.ident;
            match &variant.fields {
                Fields::Named(fields_named) => {
                    for field in &fields_named.named {
                        let field_name = &field.ident;
                        let field_access = quote! { #variant_ident { #field_name, .. } };
                        if field.attrs.is_empty() {
                            validations.push(quote! {
                                if let Self::#variant_ident { #field_name, .. } = self {
                                    fn assert_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                        value.validate()
                                    }
                                    assert_rod_validate(#field_name)?;
                                }
                            });
                        } else {
                            for attr in &field.attrs {
                                if attr.path().is_ident("rod") {
                                    if attr.parse_args::<RodNone>().is_ok() {
                                        continue;
                                    }
                                    match attr.parse_args::<RodAttr>() {
                                        Ok(rod_attr) => {
                                            let rod_attr_type = rod_attr.ty;
                                            assert_type!(field_name.as_ref(), &field.ty, rod_attr_type);
                                            let validations_for_field = match rod_attr.content {
                                                RodAttrContent::String(ref content) => content.get_validations(
                                                    quote! { #field_access }
                                                ),
                                                RodAttrContent::Integer(ref content) => content.get_validations(
                                                    quote! { #field_access }
                                                ),
                                            };
                                            for v in validations_for_field {
                                                validations.push(quote! {
                                                    if let Self::#variant_ident { #field_name, .. } = self {
                                                        #v
                                                    }
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            abort!(
                                                attr.span(), "Failed to parse attribute: {}", e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Fields::Unnamed(fields_unnamed) => {
                    let field_count = fields_unnamed.unnamed.len();
                    for (idx, field) in fields_unnamed.unnamed.iter().enumerate() {
                        let field_access = if field_count == 1 {
                            quote! { inner }
                        } else {
                            let field_index = syn::Index::from(idx);
                            quote! { inner.#field_index }
                        };
                        if field.attrs.is_empty() {
                            validations.push(quote! {
                                if let Self::#variant_ident(inner) = self {
                                    fn assert_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                        value.validate()
                                    }
                                    assert_rod_validate(#field_access)?;
                                }
                            });
                        } else {
                            for attr in &field.attrs {
                                if attr.path().is_ident("rod") {
                                    if attr.parse_args::<RodNone>().is_ok() {
                                        continue;
                                    }
                                    match attr.parse_args::<RodAttr>() {
                                        Ok(rod_attr) => {
                                            let rod_attr_type = rod_attr.ty;
                                            assert_type!(Some("<unnamed field>"), &field.ty, rod_attr_type);
                                            let validations_for_field = match rod_attr.content {
                                                RodAttrContent::String(ref content) => content.get_validations(
                                                    quote! { #field_access }
                                                ),
                                                RodAttrContent::Integer(ref content) => content.get_validations(
                                                    quote! { #field_access }
                                                ),
                                            };
                                            for v in validations_for_field {
                                                validations.push(quote! {
                                                    if let Self::#variant_ident(inner) = self {
                                                        #v
                                                    }
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            abort!(
                                                attr.span(), "Failed to parse attribute: {}", e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Fields::Unit => {
                    // No fields to validate
                }
            }
        }
    }

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


