#![feature(iter_order_by)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    braced, parse_macro_input, Data, DeriveInput, Fields, Ident, Result as SynResult, Type, TypeTuple
};
use quote::{format_ident, quote};
mod types;
use types::{RodBooleanContent, RodFloatContent, RodIntegerContent, RodLiteralContent, RodOptionContent, RodStringContent, RodTupleContent};

#[derive(Debug, Clone, PartialEq)]
enum TypeEnum {
    Type(Ident),
    Tuple(TypeTuple),
}

impl std::fmt::Display for TypeEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeEnum::Type(ident) => write!(f, "{}", ident),
            TypeEnum::Tuple(_) => write!(f, "Tuple"),
        }
    }
}

fn get_type(ty: &Type) -> Option<TypeEnum> {
    match ty {
        Type::Path(type_path) => type_path.path.segments.last().map(|s| TypeEnum::Type(s.ident.clone())),
        Type::Reference(type_ref) => get_type(type_ref.elem.as_ref()),
        Type::Tuple(tuple) => Some(TypeEnum::Tuple(tuple.clone())),
        _ => None,
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

fn recurse_rod_attr_tuple(input: &RodAttr, level: usize) -> Option<Vec<(RodAttrType, usize)>> {
    match &input.content {
        RodAttrContent::Tuple(content) => {
            let mut types = Vec::new();
            for field in &content.fields {
                if let Some(inner_types) = recurse_rod_attr_tuple(field, level + 1) {
                    types.extend(inner_types);
                } else {
                    types.push((field.ty.clone(), level));
                }
            }
            Some(types)
        }
        _ => None,
    }
}

fn recurse_tuple(ty: &Type, level: usize) -> Option<Vec<(RodAttrType, usize)>> {
    if let Type::Tuple(tuple) = ty {
        let mut types = Vec::new();
        for elem in &tuple.elems {
            match elem {
                Type::Tuple(_) => {
                    if let Some(inner_types) = recurse_tuple(elem, level + 1) {
                        types.extend(inner_types);
                    }
                }
                Type::Path(_) => {
                    let (ty, _) = recurse_type_path(elem, 0).unwrap();
                    types.push((ty, level));
                }
                _ => panic!("Unexpected type in tuple: {:?}", elem),
            }
        }
        Some(types)
    } else {
        None
    }
}

fn diff_tuple_array(
    expected: &Vec<(RodAttrType, usize)>,
    actual: &Vec<(RodAttrType, usize)>
) -> ((RodAttrType, usize), (RodAttrType, usize)) {
    let mut i = 0;
    let mut j = 0;
    while i < expected.len() && j < actual.len() {
        if expected[i].0 != actual[j].0 || expected[i].1 != actual[j].1 {
            return (expected[i].clone(), actual[j].clone());
        }
        i += 1;
        j += 1;
    }
    (expected[i].clone(), actual[j].clone())
}

macro_rules! assert_type {
    ($name:expr, $ty:expr, $expected:expr) => {
        match $expected.ty {
            RodAttrType::Option(_) => {
                let inner_type = recurse_rod_attr_opt(&$expected, 0);
                let inner_actual_type = recurse_type_path($ty, 0);
                if inner_type.is_some() && inner_type != inner_actual_type {
                    if let Some((inner_type, level)) = inner_type {
                        if let Some((inner_actual_type, actual_level)) = inner_actual_type {
                            if level != actual_level {
                            abort!(
                                $name.span(), "Expected `{}` to be a {}-nested Option, but found {}-nested Option",
                                $name, level, actual_level;
                                help = "Make sure the nesting levels match in the attribute and the type";
                            );
                            } else {
                            abort!(
                                $name.span(), "Expected `{}` to be a {} type, but found {}",
                                $name, inner_type, inner_actual_type;
                                help = "Try using {} instead of {}", inner_type.inner_type(), get_type($ty).unwrap()
                            );
                            }
                        } 
                    }
                }
            }
            RodAttrType::Tuple(_) => {
                let inner_ty_array = recurse_rod_attr_tuple(&$expected, 0);
                let inner_actual_ty_array = recurse_tuple($ty, 0);
                if inner_ty_array != inner_actual_ty_array {
                    let (i, j) = diff_tuple_array(inner_ty_array.as_ref().unwrap(), inner_actual_ty_array.as_ref().unwrap());
                    abort!(
                        $ty.span(), "`{}` is a tuple type that does not match the expected tuple type",
                        $name;
                        note = "Expected: {} at depth {}, Got: {} at depth {}",
                        i.0, i.1, j.0, j.1;
                        help = if i.1 != j.1 {
                            format!("Make sure the nesting levels match in the attribute and the type")
                        } else {
                            format!("Try using {} instead of {}", i.0.inner_type(), j.0.inner_type())
                        };
                    );
                }
            }
            _ => {
                let actual_type: RodAttrType = $ty.into();
                if actual_type != $expected.ty && !matches!($expected.ty, RodAttrType::Literal(_)) {
                    abort!(
                        $ty.span(), "Expected `{}` to be a {} type, but found {}", 
                        $name, $expected.ty, actual_type; 
                        help = "Try using {} instead of {}", $expected.ty.inner_type(), get_type($ty).unwrap()
                    );
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
                $variant(TypeEnum),
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
                let type_ident = get_type(&ty).unwrap_or_else(|| {
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
                let type_ident = get_type(ty).unwrap_or_else(|| {
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

        impl RodAttrType {
            fn inner_type(&self) -> &TypeEnum {
                match self {
                    $(
                        RodAttrType::$variant(ident) => ident,
                    )*
                }
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
    },
    Float {
        ident: Ident,
        content: RodFloatContent,
        match: ["f32", "f64"]
    },
    Tuple {
        ident: TypeTuple,
        content: RodTupleContent,
        match: ["Tuple"]
    },
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

macro_rules! get_field_validations {
    (
        $field_access:expr,
        $field:expr,
    ) => {
        $field.attrs.iter().filter_map(|attr| {
            if attr.path().is_ident("rod") {
                if attr.parse_args::<RodNone>().is_ok() {
                    return None;
                }
                match attr.parse_args::<RodAttr>() {
                    Ok(rod_attr) => {
                        assert_type!($field_access, &$field.ty, rod_attr);
                        let validations_for_field = rod_content_match!(
                            rod_attr.content,
                            $field_access,
                            [String, Integer, Literal, Boolean, Option, Float, Tuple]
                        );
                        Some(quote! {
                            #validations_for_field
                        })
                    }
                    Err(e) => {
                        abort!(
                            attr.span(), "Failed to parse attribute: {}", e
                        );
                    }
                }
            } else {
                None
            }
        })
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

    let validations: proc_macro2::TokenStream = match &ast.data {
        Data::Struct(data_struct) => {
            if let Fields::Named(fields_named) = &data_struct.fields {
                fields_named.named.iter().map(|field| {
                    let field_name = &field.ident;
                    if field.attrs.is_empty() {
                        quote! {
                            fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                value.validate()
                            }
                            assert_impl_rod_validate(field_name)?;
                        }
                    } else {
                        let validations: proc_macro2::TokenStream = get_field_validations!(
                            field_name.as_ref().unwrap(),
                            field,
                        ).collect();
                        quote! {
                            let #field_name = &self.#field_name;
                            #validations
                        }
                    }
                }).collect()
            } else {
                unreachable!()
            }
        },
        Data::Enum(data_enum) => {
            let match_arms = data_enum.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                match &variant.fields {
                    Fields::Named(fields_named) => {
                        let field_names = fields_named.named.iter().map(|f| f.ident.clone());
                        let validations_iter = fields_named.named.iter().map(|field| {
                            let field_name = &field.ident;
                            if field.attrs.is_empty() {
                                quote! {
                                    fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                        value.validate()
                                    }
                                    assert_impl_rod_validate(#field_name)?;
                                }
                            } else {
                                get_field_validations!(
                                    field_name.as_ref().unwrap(),
                                    field,
                                ).collect()
                            }
                        });
                        quote! {
                            Self::#variant_ident { #( #field_names ),* } => {
                                #(#validations_iter)*
                            }
                        }
                    }
                    Fields::Unnamed(fields_unnamed) => {
                        let field_count = fields_unnamed.unnamed.len();
                        let field_idents: Vec<syn::Ident> = (0..field_count)
                            .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                            .collect();
                        let validations_iter = fields_unnamed.unnamed.iter().enumerate().map(|(idx, field)| {
                            let field_ident = &field_idents[idx];
                            if field.attrs.is_empty() {
                                quote! {
                                    fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                                        value.validate()
                                    }
                                    assert_impl_rod_validate(#field_ident)?;
                                }
                            } else {
                                get_field_validations!(
                                    field_ident,
                                    field,
                                ).collect()
                            }
                        });
                        quote! {
                            Self::#variant_ident(#( #field_idents ),*) => {
                                #(#validations_iter)*
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            Self::#variant_ident => {}
                        }
                    }
                }
            });
            quote! {
                match self {
                    #( #match_arms )*
                }
            }
        }
        Data::Union(_) => {
            todo!("Union types are not supported yet");
        }
    };

    quote! {
        impl RodValidate for #name {
            fn validate(&self) -> Result<(), RodValidateError> {
                #validations
                Ok(())
            }
        }
    }
    .into()
}


