extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    braced, parse_macro_input, Data, DeriveInput, Fields, Ident, Result as SynResult, Type
};
use types::{RodBooleanContent, RodIntegerContent, RodLiteralContent, RodOptionContent, RodStringContent};

mod types;

#[inline(always)]
fn get_type_ident(ty: &Type) -> Option<&Ident> {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| &s.ident)
    } else {
        None
    }
}

fn recurse_rod_attr_opt(input: &RodAttr, level: usize) -> Option<(RodAttrType, usize)> {
    match &input.content {
        RodAttrContent::Option(content) => {
            if let Some(inner) = &content.inner {
                recurse_rod_attr_opt(&inner.as_ref(), level + 1)
            } else {
                None
            }
        }
        _ => {
            Some((input.ty.clone(), level))
        }
    }
}

fn recurse_type_path(ty: &Type, level: usize) -> Option<(RodAttrType, usize)> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(arg) = args.args.first() {
                    if let syn::GenericArgument::Type(ty) = arg {
                        return recurse_type_path(ty, level + 1);
                    }
                }
            } else {
                return Some((ty.into(), level));
            }
        }
    }
    None
}

macro_rules! assert_type {
    ($name:expr, $ty:expr, $expected:expr) => {
        let actual_type: RodAttrType = $ty.into();
        if actual_type != $expected.ty && !matches!($expected.ty, RodAttrType::Literal(_)) {
            abort!(
                $name.span(), "Expected `{}` to be a {} type, but found {}", 
                $name.unwrap(), $expected.ty, actual_type; 
                help = "The type of the field must match the expected type in the attribute.";
            );
        }
        if matches!($expected.ty, RodAttrType::Option(_)) {
            let inner_type = recurse_rod_attr_opt(&$expected, 0);
            let inner_actual_type = recurse_type_path($ty, 0);
            if inner_type.is_some() && inner_type != inner_actual_type {
                if let Some((inner_type, level)) = inner_type {
                    if let Some((inner_actual_type, actual_level)) = inner_actual_type {
                        if level != actual_level {
                            abort!(
                                $name.span(), "Expected `{}` to be a {}-nested Option, but found {}-nested Option",
                                $name.unwrap(), level, actual_level;
                                help = "The type of the field must match the expected type in the attribute.";
                            );
                        } else {
                            abort!(
                                $name.span(), "Expected `{}` to be a {} type, but found {}",
                                $name.unwrap(), inner_type, inner_actual_type;
                                help = "The type of the field must match the expected type in the attribute.";
                            );
                        }
                    } 
                }
            }
        }
    };
}

struct RodNone;

impl Parse for RodNone {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let none: Ident = input.parse()?;
        if none != "none" {
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

macro_rules! impl_rod_types {
    (
        $(
            $variant:ident {
                ident: $ident_ty:ty,
                content: $content_ty:ty,
                match: [$($ty_str:expr),*]
            }
        ),* $(,)?
    ) => {
        #[derive(Debug, PartialEq, Clone)]
        enum RodAttrType {
            $(
                $variant($ident_ty),
            )*
        }

        impl std::fmt::Display for RodAttrType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        RodAttrType::$variant(ident) => write!(f, concat!(stringify!($variant), "({})"), ident),
                    )*
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
                let type_str = type_ident.to_string();
                $(
                    if [$( $ty_str ),*].contains(&type_str.as_str()) {
                        return RodAttrType::$variant(type_ident.clone());
                    }
                )*
                abort!(
                    ty.span(), "Unsupported type: {}", type_ident
                );
            }
        }

        impl From<&Type> for RodAttrType {
            fn from(ty: &Type) -> RodAttrType {
                let type_ident = get_type_ident(ty).unwrap_or_else(|| {
                    abort!(
                        ty.span(), "Expected a type path, but found: {:?}", ty
                    );
                });
                let type_str = type_ident.to_string();
                $(
                    if [$( $ty_str ),*].contains(&type_str.as_str()) {
                        return RodAttrType::$variant(type_ident.clone());
                    }
                )*
                abort!(
                    ty.span(), "Unsupported type: {}", type_ident
                );
            }
        }

        enum RodAttrContent {
            $(
                $variant($content_ty),
            )*
        }

        impl Parse for RodAttr {
            fn parse(input: ParseStream) -> SynResult<Self> {
                let ty: Type = input.parse()?;
                let rod_type: RodAttrType = ty.into();
                let inner;
                braced!(inner in input);
                let content = match rod_type {
                    $(
                        RodAttrType::$variant(_) => {
                            let content: $content_ty = inner.parse()?;
                            RodAttrContent::$variant(content)
                        }
                    ),*
                };
                Ok(RodAttr { ty: rod_type, content })
            }
        }
    }
}

impl_rod_types! {
    String {
        ident: Ident,
        content: RodStringContent,
        match: ["String", "str", "OsString", "OsStr", "PathBuf", "Path", "Cow"]
    },
    Integer {
        ident: Ident,
        content: RodIntegerContent,
        match: ["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize"]
    },
    Literal {
        ident: Ident,
        content: RodLiteralContent,
        match: ["Literal"]
    },
    Boolean {
        ident: Ident,
        content: RodBooleanContent,
        match: ["bool"]
    },
    Option {
        ident: Ident,
        content: RodOptionContent,
        match: ["Option"]
    }
}

macro_rules! rod_content_match {
    ($content:expr, $field_access:expr, [ $( $variant:ident ),* ]) => {
        match $content {
            $(
                RodAttrContent::$variant(ref content) => content.get_validations($field_access),
            )*
        }
    };
}

/// Derives the `RodValidate` trait for a struct.
///
/// Implements validation logic for struct fields annotated with `#[rod(...)]`.
/// Fields without the attribute are required to implement `RodValidate`.
/// Many standard types are supported, including [`RodStringContent`][crate::types::RodStringContent], [`RodIntegerContent`][crate::types::RodIntegerContent], [`RodLiteralContent`][crate::types::RodLiteralContent], [`RodBooleanContent`][crate::types::RodBooleanContent], and [`RodOptionContent`][crate::types::RodOptionContent]. 
/// To see the available attributes, refer to the documentation for each type.
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
///         }
///     )]
///     username: String,
///     #[rod(
///         u8 {
///             size: 18..=99, // Age between 18 and 99
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
                        fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                            value.validate()
                        }
                        assert_impl_rod_validate(&self.#field_name)?;
                    });
                } else {
                    for attr in &field.attrs {
                        if attr.path().is_ident("rod") {
                            if attr.parse_args::<RodNone>().is_ok() {
                                continue;
                            }
                            match attr.parse_args::<RodAttr>() {
                                Ok(rod_attr) => {
                                    assert_type!(field_name.as_ref(), &field.ty, rod_attr);
                                    let validations_for_field = rod_content_match!(
                                        rod_attr.content,
                                        quote! { self.#field_name },
                                        [String, Integer, Literal, Boolean, Option]
                                    );
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
                                    fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                        value.validate()
                                    }
                                    assert_impl_rod_validate(#field_name)?;
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
                                            assert_type!(Some("<unnamed field>"), &field.ty, rod_attr);
                                            let validations_for_field = rod_content_match!(
                                                rod_attr.content,
                                                quote! { #field_access },
                                                [String, Integer, Literal, Boolean, Option]
                                            );
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
                                    fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                        value.validate()
                                    }
                                    assert_impl_rod_validate(#field_access)?;
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
                                            assert_type!(Some("<unnamed field>"), &field.ty, rod_attr);
                                            let validations_for_field = rod_content_match!(
                                                rod_attr.content,
                                                quote! { #field_access },
                                                [String, Integer, Literal, Boolean, Option]
                                            );
                                            for v in validations_for_field {
                                                validations.push(quote! {
                                                    if let &Self::#variant_ident(inner) = self {
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


