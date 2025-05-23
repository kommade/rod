#![feature(iter_order_by)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::{abort, emit_warning, proc_macro_error};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    braced, parse_macro_input, Data, DeriveInput, Fields, Ident, Result as SynResult, Type, TypeTuple
};
use quote::quote;
mod types;
use types::{CustomContent, RodBooleanContent, RodFloatContent, RodIntegerContent, RodLiteralContent, RodOptionContent, RodSkipContent, RodStringContent, RodTupleContent};

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

#[derive(Debug, Clone, PartialEq)]
enum IsNestedReference {
    None,
    Single,
    More,
}

fn type_is_nested_reference(ty: &Type) -> IsNestedReference {
    match ty {
        Type::Reference(type_ref) => {
            if let Type::Reference(_) = type_ref.elem.as_ref() {
                IsNestedReference::More
            } else {
                IsNestedReference::Single
            }
        }
        _ => IsNestedReference::None,
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
                debug_assert!(<&syn::Type as TryInto<RodAttrType>>::try_into(ty).is_ok(), "Expected a valid rod type, but found: {:?}", ty);
                let attr_ty = ty.try_into().ok()?;
                return Some((attr_ty, level));
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
                Type::Reference(type_ref) => {
                    if let Some((ty, _)) = recurse_type_path(&type_ref.elem, 0) {
                        types.push((ty, level));
                    }
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

fn recurse_iterable(
    input: &RodAttr,
    level: usize
) -> Option<(RodAttrType, usize)> {
    match &input.content {
        RodAttrContent::Iterable(content) => {
            recurse_iterable(content.item.as_ref(), level + 1)
        }
        _ => Some((input.ty.clone(), level)),
    }
}

macro_rules! assert_type {
    ($name:expr, $ty:expr, $expected:expr) => {
        match $expected.ty {
            RodAttrType::Iterable(_) => {
                let item_type = recurse_iterable(&$expected, 0);
                let item_actual_type = recurse_type_path($ty, 0);
                if item_type.is_some() && item_type != item_actual_type {
                    if let Some((item_type, level)) = item_type {
                        if let Some((item_actual_type, actual_level)) = item_actual_type {
                            if level != actual_level {
                                abort!(
                                    $name.span(), "Expected `{}` to be a {}-nested Iterable, but found {}-nested Iterable",
                                    $name, level, actual_level;
                                    help = "Make sure the nesting levels match in the attribute and the type";
                                );
                            } else {
                                abort!(
                                    $name.span(), "Expected `{}` to be a {} type, but found {}",
                                    $name, item_type, item_actual_type;
                                    help = "Try using {} instead of {}", item_type.inner_type(), get_type($ty).unwrap()
                                );
                            }
                        }
                    }
                }
            },
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
                debug_assert!(inner_ty_array.is_some() && inner_actual_ty_array.is_some(), "Expected a tuple type, but found: {:?}", $ty);
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
            RodAttrType::Skip(_) => {
                // ignore
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
        #[derive(Debug, Clone)]
        enum RodAttrType {
            $(
                $variant(TypeEnum),
            )*
        }

        impl PartialEq for RodAttrType {
            fn eq(&self, other: &Self) -> bool {
                #[allow(unreachable_patterns)]
                match (self, other) {
                    (RodAttrType::Skip(_), _) => true,
                    (_, RodAttrType::Skip(_)) => true,
                    $(
                        (RodAttrType::$variant(ident1), RodAttrType::$variant(ident2)) => ident1 == ident2,
                    )*
                    _ => false,
                }
            }
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
            fn from(ty: Type) -> Self {
                let type_ident = get_type(&ty).unwrap_or_else(|| {
                    #[cfg(debug_assertions)]
                    abort!(
                        ty.span(), "Expected a type path, reference or tuple, but found: {:?}", ty
                    );
                    #[cfg(not(debug_assertions))]
                    abort!(
                        ty.span(), "Unsupported type",
                    );
                });
                let type_str = type_ident.to_string();
                $(
                    if [$( $ty_str ),*].contains(&type_str.as_str()) {
                        return RodAttrType::$variant(type_ident.clone());
                    }
                )*
                return RodAttrType::Custom(type_ident);
            }
        }

        impl From<&Type> for RodAttrType {
            fn from(ty: &Type) -> Self {
                let type_ident = get_type(ty).unwrap_or_else(|| {
                    #[cfg(debug_assertions)]
                    abort!(
                        ty.span(), "Expected a type path, reference or tuple, but found: {:?}", ty
                    );
                    #[cfg(not(debug_assertions))]
                    abort!(
                        ty.span(), "Unsupported type",
                    );
                });
                let type_str = type_ident.to_string();
                $(
                    if [$( $ty_str ),*].contains(&type_str.as_str()) {
                        return RodAttrType::$variant(type_ident.clone());
                    }
                )*
                return RodAttrType::Custom(type_ident);
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
            fn type_is_valid_rod_type(ty: &Type) -> bool {
                #[allow(unreachable_patterns)]
                match ty.into() {
                    RodAttrType::Skip(_) | RodAttrType::Custom(_) => false,
                    $(
                        RodAttrType::$variant(_) => true,
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
                #[allow(unreachable_patterns)]
                let content = match rod_type {
                    RodAttrType::Skip(_) => {
                        let skip: RodSkipContent = input.parse()?;
                        RodAttrContent::Skip(skip)
                    }
                    RodAttrType::Custom(_) => {
                        let content: CustomContent = input.parse()?;
                        RodAttrContent::Custom(content)
                    }
                    $(
                        RodAttrType::$variant(_) => {
                            braced!(inner in input);
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
    Skip {
        ident: Ident,
        content: RodSkipContent,
        match: ["Skip", "skip"]
    },
    Custom {
        ident: Ident,
        content: CustomContent,
        match: []
    },
    Iterable {
        ident: Ident,
        content: types::RodIterableContent,
        match: ["Iterable"]
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

macro_rules!  get_field_validations {
    (
        $field_access:expr,
        $field:expr,
    ) => {
        $field.attrs.iter().filter_map(|attr| {
            if attr.path().is_ident("rod") {
                match attr.parse_args::<RodAttr>() {
                    Ok(rod_attr) => {
                        assert_type!($field_access, &$field.ty, rod_attr);
                        let validations_for_field = rod_content_match!(
                            rod_attr.content,
                            $field_access,
                            [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
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

macro_rules! check_valid_rod_type {
    ($ty:expr, $span:expr, $field_name:expr) => {
        if RodAttrType::type_is_valid_rod_type(&$ty) {
            let valid_type = get_type(&$ty).unwrap();
            emit_warning!(
                $span,
                "Field `{}` has no `#[rod(...)]` attribute, however it is of type `{}` which is a valid Rod type.",
                $field_name.as_ref().unwrap(), valid_type;
                help = "If you want to validate this field, add a `#[rod({}{{...}})]` attribute to it.\nIf you want to skip validation, use `#[rod(Skip)]`.",
                valid_type
            )
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
/// use rod::prelude::*;
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
/// use rod::prelude::*;
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
                    // If no attributes are present, we assume it's a custom type that implements `RodValidate`
                    // If a custom type appears inside a Rod type, it has to be explicitly annotated with `#[rod(...CustomType...)]`
                    // The name of the custom type and the annotation must match
                    // Otherwise, the custom type can just have no #rod attribute
                    if field.attrs.is_empty() {
                        check_valid_rod_type!(field.ty, field.ty.span(), field_name);
                        quote! {
                            let #field_name = &self.#field_name;
                            assert_impl_rod_validate(#field_name)?;
                        }
                    } else {
                        let validations: proc_macro2::TokenStream = get_field_validations!(
                            field_name.as_ref().unwrap(),
                            field,
                        ).collect();
                        match type_is_nested_reference(&field.ty) {
                            IsNestedReference::None => quote! {
                                let #field_name = &self.#field_name;
                                #validations
                            },
                            IsNestedReference::Single => quote! {
                                let #field_name = self.#field_name;
                                #validations
                            },
                            IsNestedReference::More => {
                                // If the field is a reference to a reference, we cannot validate it directly
                                // because it would require dereferencing, which would require the type to be `Copy` or `Deref`.
                                // Maybe we should allow this in the future, but for now we just abort.
                                abort!(
                                    field.ty.span(), "Field `{}` is a reference to a reference, which is not supported.", field_name.as_ref().unwrap();
                                    help = "Use a single reference instead, e.g. `&T` instead of `&&T`."
                                )
                            }
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
                            if type_is_nested_reference(&field.ty) == IsNestedReference::More {
                                abort!(
                                    field.ty.span(), "Field `{}` is a reference to a reference, which is not supported.", field_name.as_ref().unwrap();
                                    help = "Use a single reference instead, e.g. `&T` instead of `&&T`."
                                )
                            }
                            if field.attrs.is_empty() {
                                check_valid_rod_type!(field.ty, field.ty.span(), field_name);
                                quote! {
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
                            let field_ident = field_idents.get(idx);
                            if type_is_nested_reference(&field.ty) == IsNestedReference::More {
                                abort!(
                                    field.ty.span(), "Field {} of variant `{}` is a reference to a reference, which is not supported.", idx, variant.ident;
                                    help = "Use a single reference instead, e.g. `&T` instead of `&&T`."
                                )
                            }
                            if field.attrs.is_empty() {
                                check_valid_rod_type!(field.ty, field.ty.span(), field_ident);
                                quote! {
                                    assert_impl_rod_validate(#field_ident)?;
                                }
                            } else {
                                get_field_validations!(
                                    field_ident.as_ref().unwrap(),
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
                fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateError> {
                    value.validate()
                }
                #validations
                Ok(())
            }
        }
    }
    .into()
}


