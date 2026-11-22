extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::{abort, emit_warning, proc_macro_error};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, ExprClosure, Fields, Ident, LitStr, Result as SynResult, Type, TypeTuple,
    parse_macro_input,
};
mod schema;
mod types;
use types::{
    CustomContent, RodBooleanContent, RodFloatContent, RodIntegerContent, RodLiteralContent,
    RodOptionContent, RodSkipContent, RodStringContent, RodTupleContent,
};

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
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|s| TypeEnum::Type(s.ident.clone())),
        Type::Reference(type_ref) => get_type(type_ref.elem.as_ref()),
        Type::Tuple(tuple) => Some(TypeEnum::Tuple(tuple.clone())),
        _ => None,
    }
}

/// Number of `&` layers wrapping the innermost type of `ty`. Any depth is
/// supported: bindings are normalized to a single reference before checks run.
fn reference_depth(ty: &Type) -> usize {
    let mut depth = 0;
    let mut current = ty;
    while let Type::Reference(type_ref) = current {
        depth += 1;
        current = type_ref.elem.as_ref();
    }
    depth
}

/// The type underneath all reference layers (`&&&T` -> `T`).
fn innermost_type(ty: &Type) -> Type {
    match ty {
        Type::Reference(type_ref) => innermost_type(type_ref.elem.as_ref()),
        other => other.clone(),
    }
}

/// A one-reference type expression for the innermost type (`&&str` -> `&str`).
/// This is the type every generated check (and custom closure) receives.
fn single_ref_type(ty: &Type) -> Type {
    syn::Type::Reference(syn::TypeReference {
        and_token: Default::default(),
        lifetime: None,
        mutability: None,
        elem: Box::new(innermost_type(ty)),
    })
}

/// A `**...*` dereference prefix peeling `count` reference layers.
fn deref_prefix(count: usize) -> proc_macro2::TokenStream {
    let mut stars = proc_macro2::TokenStream::new();
    for _ in 0..count {
        stars.extend(std::iter::once(proc_macro2::TokenTree::Punct(
            proc_macro2::Punct::new('*', proc_macro2::Spacing::Joint),
        )));
    }
    stars
}

/// Struct-style binding for a field access, normalized so that `access` has
/// type `&T_inner` regardless of how many references wrap the field:
/// `T` -> `&self.field`, `&T` (or `&&T`, `&&&T`, ...) -> peels down to `&T`.
/// Peeling is always possible because `&T: Copy` for every `T`.
fn struct_field_binding(access: &Ident, field: &syn::Field) -> proc_macro2::TokenStream {
    let depth = reference_depth(&field.ty);
    if depth == 0 {
        quote! {
            let #access = &self.#access;
        }
    } else {
        let stars = deref_prefix(depth - 1);
        quote! {
            let #access = #stars self.#access;
        }
    }
}

/// Enum pattern bindings add one reference layer (default binding modes), so
/// a field of type `&&T` arrives as `&&&T`. Rebind with `depth` peels to
/// normalize to `&T_inner`. Returns empty tokens when no peeling is needed.
fn enum_field_binding(access: &Ident, field: &syn::Field) -> proc_macro2::TokenStream {
    let depth = reference_depth(&field.ty);
    if depth == 0 {
        quote! {}
    } else {
        let stars = deref_prefix(depth);
        quote! {
            let #access = #stars #access;
        }
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
        _ => Some((input.ty.clone(), level)),
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
                debug_assert!(
                    <&syn::Type as TryInto<RodAttrType>>::try_into(ty).is_ok(),
                    "Expected a valid rod type, but found: {:?}",
                    ty
                );
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
    actual: &Vec<(RodAttrType, usize)>,
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

fn recurse_iterable(input: &RodAttr, level: usize) -> Option<(RodAttrType, usize)> {
    match &input.content {
        RodAttrContent::Iterable(content) => recurse_iterable(content.item.as_ref(), level + 1),
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

enum RodExpr {
    Attribute(RodAttr),
    Check(RodCheck),
    Message(RodMessage),
    Group(RodGroup),
    AsyncCheck(RodAsyncCheck),
}

impl Parse for RodExpr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        if input.peek(Ident) && input.peek2(syn::Token![=]) {
            let ident: Ident = input.fork().parse()?;
            match ident.to_string().as_str() {
                "group" => {
                    let rod_group: RodGroup = input.parse()?;
                    Ok(RodExpr::Group(rod_group))
                }
                "async_check" => {
                    let rod_async_check: RodAsyncCheck = input.parse()?;
                    Ok(RodExpr::AsyncCheck(rod_async_check))
                }
                _ => {
                    let rod_check: RodCheck = input.parse()?;
                    Ok(RodExpr::Check(rod_check))
                }
            }
        } else if input.peek(Ident) && input.peek2(syn::Token![:]) {
            let rod_message: RodMessage = input.parse()?;
            Ok(RodExpr::Message(rod_message))
        } else {
            let rod_attr: RodAttr = input.parse()?;
            Ok(RodExpr::Attribute(rod_attr))
        }
    }
}

/// Extracts the `group = "..."` tag and the primary validation type from a
/// field's `#[rod(...)]` attributes. Used for metadata/schema generation
/// (the validation codegen re-parses attrs independently).
fn parse_field_rod(field: &syn::Field) -> (Option<LitStr>, Option<RodAttr>) {
    let mut group: Option<LitStr> = None;
    let mut rod_attr: Option<RodAttr> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("rod") {
            continue;
        }
        let exprs = attr.parse_args_with(
            syn::punctuated::Punctuated::<RodExpr, syn::Token![,]>::parse_terminated,
        );
        if let Ok(exprs) = exprs {
            for expr in exprs {
                match expr {
                    RodExpr::Group(g) => group = Some(g.group),
                    RodExpr::Attribute(a) => rod_attr = Some(a),
                    _ => {}
                }
            }
        }
    }
    (group, rod_attr)
}

/// Whether a validated field participates unconditionally: everything except
/// `#[rod(Skip)]` and `#[rod(Option { ... })]` fields does.
fn field_required(rod_attr: &Option<RodAttr>) -> bool {
    match rod_attr {
        None => true,
        Some(attr) => !matches!(
            &attr.content,
            RodAttrContent::Skip(_) | RodAttrContent::Option(_)
        ),
    }
}

/// Whether any field of the type carries a `group = "..."` tag.
fn has_groups(ast: &DeriveInput) -> bool {
    let fields: Vec<&syn::Field> = match &ast.data {
        Data::Struct(data) => data.fields.iter().collect(),
        Data::Enum(data) => data.variants.iter().flat_map(|v| v.fields.iter()).collect(),
        Data::Union(_) => return false,
    };
    fields.iter().any(|f| parse_field_rod(f).0.is_some())
}

/// Extracts `async_check = ...` closures plus the field's optional
/// `message: "..."` override from a field's `#[rod(...)]` attributes.
/// Returns `(closures, message)`; `closures` may be empty.
fn parse_field_async_checks(field: &syn::Field) -> (Vec<ExprClosure>, Option<LitStr>) {
    let mut checks: Vec<ExprClosure> = Vec::new();
    let mut message: Option<LitStr> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("rod") {
            continue;
        }
        let exprs = attr.parse_args_with(
            syn::punctuated::Punctuated::<RodExpr, syn::Token![,]>::parse_terminated,
        );
        if let Ok(exprs) = exprs {
            for expr in exprs {
                match expr {
                    RodExpr::AsyncCheck(c) => checks.push(c.closure),
                    RodExpr::Message(m) => message = Some(m.message),
                    _ => {}
                }
            }
        }
    }
    (checks, message)
}

/// Whether any field of the type carries an `async_check` closure.
fn has_async_checks(ast: &DeriveInput) -> bool {
    let fields: Vec<&syn::Field> = match &ast.data {
        Data::Struct(data) => data.fields.iter().collect(),
        Data::Enum(data) => data.variants.iter().flat_map(|v| v.fields.iter()).collect(),
        Data::Union(_) => return false,
    };
    fields
        .iter()
        .any(|f| !parse_field_async_checks(f).0.is_empty())
}

/// Generates one awaited `async_check` invocation. Assumes the field value is
/// already bound to `access` and a `rod_validate_async_errors` vec is in
/// scope. The closure must return a `Send` future resolving to `bool`.
fn async_check_tokens(
    access: &Ident,
    closure: &ExprClosure,
    message: Option<&LitStr>,
) -> proc_macro2::TokenStream {
    let path = access.to_string();
    let error_expr = match message {
        Some(msg) => quote! { ::rod_validation::RodValidateError::UserDefined(#msg.to_string()) },
        None => quote! { ::rod_validation::RodValidateError::CheckFailed(#path) },
    };
    quote! {
        if !(#closure)(#access).await {
            rod_validate_async_errors.push(#error_expr);
        }
    }
}

/// Generates `impl AsyncValidator for #name`, overriding
/// `run_async_checks` when the type declares `async_check` fields. Sync
/// rules never await; async checks are only executed by `validate_async`.
fn build_async_checks_impl(ast: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    if !has_async_checks(ast) {
        // No async checks: an empty impl relies on the trait defaults, whose
        // `run_async_checks` simply reports "nothing".
        return quote! {
            impl ::rod_validation::AsyncValidator for #name {}
        };
    }

    let body: proc_macro2::TokenStream = match &ast.data {
        Data::Struct(data) => {
            let Fields::Named(fields_named) = &data.fields else {
                unreachable!() // structs must be named (enforced by sync codegen)
            };
            let mut code = quote! {};
            for field in &fields_named.named {
                let (checks, message) = parse_field_async_checks(field);
                if checks.is_empty() {
                    continue;
                }
                let access = field.ident.as_ref().unwrap();
                let pre_bind = struct_field_binding(access, field);
                let mut field_checks = quote! {};
                for closure in &checks {
                    field_checks.extend(async_check_tokens(access, closure, message.as_ref()));
                }
                code.extend(quote! { #pre_bind #field_checks });
            }
            code
        }
        Data::Enum(data_enum) => {
            let arms = data_enum.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let variant_checks: proc_macro2::TokenStream = match &variant.fields {
                    Fields::Named(fields_named) => {
                        let mut checks = quote! {};
                        for field in &fields_named.named {
                            let (closures, message) = parse_field_async_checks(field);
                            if closures.is_empty() {
                                continue;
                            }
                            let access = field.ident.as_ref().unwrap();
                            checks.extend(enum_field_binding(access, field));
                            for closure in &closures {
                                checks.extend(async_check_tokens(access, closure, message.as_ref()));
                            }
                        }
                        checks
                    }
                    Fields::Unnamed(fields_unnamed) => {
                        let bindings: Vec<syn::Ident> = (0..fields_unnamed.unnamed.len())
                            .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                            .collect();
                        let mut checks = quote! {};
                        for (idx, field) in fields_unnamed.unnamed.iter().enumerate() {
                            let (closures, message) = parse_field_async_checks(field);
                            if closures.is_empty() {
                                continue;
                            }
                            let access = &bindings[idx];
                            checks.extend(enum_field_binding(access, field));
                            for closure in &closures {
                                checks.extend(async_check_tokens(access, closure, message.as_ref()));
                            }
                        }
                        checks
                    }
                    Fields::Unit => quote! {},
                };
                match &variant.fields {
                    Fields::Named(fields_named) => {
                        let bindings = fields_named.named.iter().map(|f| f.ident.clone());
                        quote! {
                            Self::#variant_ident { #( #bindings ),* } => { #variant_checks }
                        }
                    }
                    Fields::Unnamed(fields_unnamed) => {
                        let bindings: Vec<syn::Ident> = (0..fields_unnamed.unnamed.len())
                            .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                            .collect();
                        quote! {
                            Self::#variant_ident(#( #bindings ),*) => { #variant_checks }
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
                    #( #arms )*
                }
            }
        }
        Data::Union(_) => quote! {},
    };

    quote! {
        impl ::rod_validation::AsyncValidator for #name {
            fn run_async_checks(&self) -> impl ::core::future::Future<Output = Vec<::rod_validation::RodValidateError>> + Send {
                async move {
                    #[allow(unused_mut)]
                    let mut rod_validate_async_errors: Vec<::rod_validation::RodValidateError> = Vec::new();
                    #body
                    rod_validate_async_errors
                }
            }
        }
    }
}

struct RodAttr {
    ty: RodAttrType,
    content: RodAttrContent,
    span: proc_macro2::Span,
}

struct RodCheck {
    closure: ExprClosure,
    span: proc_macro2::Span,
}

/// Parsed `async_check = |x| ...` attribute. The closure must return a future
/// resolving to `bool`; failing futures are collected as validation errors by
/// `AsyncValidator::validate_async`.
struct RodAsyncCheck {
    closure: ExprClosure,
    span: proc_macro2::Span,
}

/// Parsed `group = "name"` attribute. Fields tagged with a group are only
/// validated by `RodValidate::validate_group` when that group is requested;
/// they are still validated by `validate`, `validate_all` and
/// `validate_all_groups`.
struct RodGroup {
    group: LitStr,
    span: proc_macro2::Span,
}

impl Parse for RodGroup {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ident = input.parse::<Ident>()?;
        if ident != "group" {
            abort!(
                ident.span(),
                "Unknown attribute `{}`. Expected `check` or `group`",
                ident
            )
        }
        input.parse::<syn::Token![=]>()?;
        let group: LitStr = input.parse()?;
        let span = ident
            .span()
            .join(group.span())
            .unwrap_or_else(|| proc_macro2::Span::call_site());
        Ok(RodGroup { group, span })
    }
}

impl Parse for RodCheck {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ident = input.parse::<Ident>()?;
        if ident != "check" {
            abort!(
                ident.span(),
                "Unknown attribute `{}`. Expected `check`",
                ident
            )
        }
        input.parse::<syn::Token![=]>()?;
        let expr: ExprClosure = input.parse()?;
        let span = ident
            .span()
            .join(expr.span())
            .unwrap_or_else(|| proc_macro2::Span::call_site());
        if expr.inputs.len() != 1 {
            abort!(
                expr.span(), "Expected a single argument for `check` closure, but found {} arguments",
                expr.inputs.len();
                help = "Make sure the closure has exactly one argument"
            );
        }
        Ok(RodCheck {
            closure: expr,
            span,
        })
    }
}

impl Parse for RodAsyncCheck {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ident = input.parse::<Ident>()?;
        if ident != "async_check" {
            abort!(
                ident.span(),
                "Unknown attribute `{}`. Expected `check` or `group`",
                ident
            )
        }
        input.parse::<syn::Token![=]>()?;
        let expr: ExprClosure = input.parse()?;
        let span = ident
            .span()
            .join(expr.span())
            .unwrap_or_else(|| proc_macro2::Span::call_site());
        if expr.inputs.len() != 1 {
            abort!(
                expr.span(), "Expected a single argument for `async_check` closure, but found {} arguments",
                expr.inputs.len();
                help = "Make sure the closure has exactly one argument"
            );
        }
        Ok(RodAsyncCheck {
            closure: expr,
            span,
        })
    }
}

struct RodMessage {
    message: LitStr,
    span: proc_macro2::Span,
}

impl Parse for RodMessage {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ident = input.parse::<Ident>()?;
        if ident != "message" {
            abort!(
                ident.span(),
                "Unknown attribute `{}`. Expected `message`",
                ident
            )
        }
        input.parse::<syn::Token![:]>()?;
        let message: LitStr = input.parse()?;
        let span = ident
            .span()
            .join(message.span())
            .unwrap_or_else(|| proc_macro2::Span::call_site());
        Ok(RodMessage { message, span })
    }
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
                let ty: Type = input.parse().unwrap_or_else(|_| {
                    abort!(
                        input.span(), "Expected a supported type, but found: {}", input
                    );
                });
                let span = ty.span();
                let rod_type: RodAttrType = ty.into();
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
                            let content: $content_ty = input.parse()?;
                            RodAttrContent::$variant(content)
                        }
                    ),*
                };
                Ok(RodAttr { ty: rod_type, content, span })
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

macro_rules!  get_field_validations {
    (
        $field_access:expr,
        $field:expr,
        $wrap_return:expr
    ) => {
        $field.attrs.iter().filter_map(|attr| {
            if attr.path().is_ident("rod") {
                let mut check_opt = None;
                let mut rod_attr_opt = None;
                let mut message_opt = None;
                let mut group_opt: Option<LitStr> = None;
                let mut async_check_opt = None;
                match attr.parse_args_with(syn::punctuated::Punctuated::<RodExpr, syn::Token![,]>::parse_terminated) {
                    Ok(exprlist) => {
                        for expr in exprlist {
                            match expr {
                                RodExpr::Check(check) => {
                                    if check_opt.is_some() {
                                        abort!(
                                            check.span, "Multiple `check` attributes found on field `{}`", $field_access;
                                            help = "Remove the extra `check` attributes"
                                        );
                                    }
                                    check_opt = Some(check);
                                }
                                RodExpr::AsyncCheck(async_check) => {
                                    if async_check_opt.is_some() {
                                        abort!(
                                            async_check.span, "Multiple `async_check` attributes found on field `{}`", $field_access;
                                            help = "Remove the extra `async_check` attributes"
                                        );
                                    }
                                    async_check_opt = Some(async_check);
                                }
                                RodExpr::Attribute(rod_attr) => {
                                    if rod_attr_opt.is_some() {
                                        abort!(
                                            rod_attr.span, "Multiple type attributes found on field `{}`", $field_access;
                                            help = "Remove the extra attributes"
                                        );
                                    }
                                    rod_attr_opt = Some(rod_attr);
                                }
                                RodExpr::Message(message) => {
                                    if message_opt.is_some() {
                                        abort!(
                                            message.span, "Multiple `message` attributes found on field `{}`", $field_access;
                                            help = "Remove the extra `message` attributes"
                                        );
                                    }
                                    message_opt = Some(message);
                                }
                                RodExpr::Group(group) => {
                                    if group_opt.is_some() {
                                        abort!(
                                            group.span, "Multiple `group` attributes found on field `{}`", $field_access;
                                            help = "Remove the extra `group` attributes"
                                        );
                                    }
                                    group_opt = Some(group.group);
                                }
                            }
                        }
                    },
                    Err(e) => {
                        abort!(
                            e.span(), "Failed to parse attribute: {}", e
                        );
                    }
                }
                match rod_attr_opt {
                    Some(rod_attr) => {
                        assert_type!($field_access, &$field.ty, rod_attr);
                        let validations_for_field = if let Some(message) = message_opt.as_ref() {
                            rod_content_match!(
                                &rod_attr.content, 
                                $field_access, 
                                $wrap_return, 
                                &message.message, 
                                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
                            )
                        } else {
                            rod_content_match!(
                                &rod_attr.content, 
                                $field_access, 
                                $wrap_return, 
                                [String, Integer, Literal, Boolean, Option, Float, Tuple, Skip, Custom, Iterable]
                            )
                        };
                        let check = check_opt.map_or_else(|| quote! {}, |check| {
                            if matches!(rod_attr.ty, RodAttrType::Skip(_)) {
                                abort!(
                                    check.span, "Cannot use `check` with `skip` attribute on field `{}`", $field_access;
                                    help = "Remove the `check` attribute"
                                );
                            }
                            let closure = &check.closure;
                            let field_type = single_ref_type(&$field.ty);
                            let path = $field_access.to_string();
                            let ret = if let Some(message) = message_opt.as_ref() {
                                let msg = &message.message;
                                $wrap_return(quote! { RodValidateError::UserDefined(#msg.to_string()) })
                            } else {
                                $wrap_return(quote! { RodValidateError::CheckFailed(#path) })
                            };
                            let field_access = $field_access;
                            quote! {
                                let check: fn(#field_type) -> bool = #closure;
                                if !check(#field_access) {
                                    #ret;
                                }
                            }
                        });
                        Some(quote! {
                            #check
                            #validations_for_field
                        })
                    }
                    None => {
                        // A field may carry only a `group`, `check` or
                        // `async_check` without a rod validation type: it is then
                        // validated by delegating to its own `RodValidate`
                        // implementation (like unattributed fields), plus running
                        // the optional `check`. `async_check` steps are executed
                        // separately by the generated `AsyncValidator` impl.
                        if group_opt.is_none() && check_opt.is_none() && async_check_opt.is_none() {
                            abort!(
                                attr.span(), "Failed to parse attribute",
                            );
                        }
                        let field_type = single_ref_type(&$field.ty);
                        let field_access = $field_access;
                        let path = field_access.to_string();
                        let ret = $wrap_return(quote! { e });
                        let mut code = quote! {
                            let assert = assert_impl_rod_validate(#field_access);
                            if let Err(errs) = assert {
                                for e in errs {
                                    #ret;
                                }
                            }
                        };
                        if let Some(check) = check_opt {
                            let closure = &check.closure;
                            let ret = if let Some(message) = message_opt.as_ref() {
                                let msg = &message.message;
                                $wrap_return(quote! { RodValidateError::UserDefined(#msg.to_string()) })
                            } else {
                                $wrap_return(quote! { RodValidateError::CheckFailed(#path) })
                            };
                            let ret = ret;
                            code.extend(quote! {
                                let check: fn(#field_type) -> bool = #closure;
                                if !check(#field_access) {
                                    #ret;
                                }
                            });
                        }
                        Some(code)
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

fn field_body(
    field: &syn::Field,
    access: &Ident,
    pre_bind: proc_macro2::TokenStream,
    wrap_validations: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    guard: bool,
) -> proc_macro2::TokenStream {
    // If no attributes are present, we assume it's a custom type that implements `RodValidate`
    // If a custom type appears inside a Rod type, it has to be explicitly annotated with `#[rod(...CustomType...)]`
    // The name of the custom type and the annotation must match
    // Otherwise, the custom type can just have no #rod attribute
    let body: proc_macro2::TokenStream = if field.attrs.is_empty() {
        check_valid_rod_type!(field.ty, field.ty.span(), Some(access));
        let ret = wrap_validations(quote! { e });
        quote! {
            let assert = assert_impl_rod_validate(#access);
            if let Err(errs) = assert {
                for e in errs {
                    #ret;
                }
            }
        }
    } else {
        get_field_validations!(access, field, wrap_validations).collect::<proc_macro2::TokenStream>()
    };

    let (group_opt, _) = parse_field_rod(field);
    let code = quote! { #pre_bind #body };
    match (guard, group_opt) {
        (true, Some(group_lit)) => quote! {
            if rod_group == #group_lit {
                #code
            }
        },
        _ => code,
    }
}

/// Walks the derive input (struct fields or enum variant fields) and produces
/// the full validation body. `guard` enables group-based `if rod_group == ...`
/// guards (used by `RodValidate::validate_group`).
fn build_validation_code(
    ast: &DeriveInput,
    wrap_validations: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    guard: bool,
) -> proc_macro2::TokenStream {
    match &ast.data {
        Data::Struct(data_struct) => {
            if let Fields::Named(fields_named) = &data_struct.fields {
                fields_named.named.iter().map(|field| {
                    let access = field.ident.as_ref().unwrap();
                    let pre_bind = struct_field_binding(access, field);
                    field_body(field, access, pre_bind, wrap_validations, guard)
                }).collect()
            } else {
                unreachable!()
            }
        }
        Data::Enum(data_enum) => {
            let match_arms = data_enum.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let body: proc_macro2::TokenStream = match &variant.fields {
                    Fields::Named(fields_named) => fields_named.named.iter().map(|field| {
                        let access = field.ident.as_ref().unwrap();
                        // Peel the extra reference layer added by the match
                        // pattern binding so checks see `&T_inner`.
                        let pre_bind = enum_field_binding(access, field);
                        field_body(field, access, pre_bind, wrap_validations, guard)
                    }).collect(),
                    Fields::Unnamed(fields_unnamed) => {
                        let field_count = fields_unnamed.unnamed.len();
                        let field_idents: Vec<syn::Ident> = (0..field_count)
                            .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                            .collect();
                        fields_unnamed.unnamed.iter().enumerate().map(|(idx, field)| {
                            let access = &field_idents[idx];
                            let pre_bind = enum_field_binding(access, field);
                            field_body(field, access, pre_bind, wrap_validations, guard)
                        }).collect()
                    }
                    Fields::Unit => quote! {},
                };
                match &variant.fields {
                    Fields::Named(fields_named) => {
                        let field_names = fields_named.named.iter().map(|f| f.ident.clone());
                        quote! {
                            Self::#variant_ident { #( #field_names ),* } => {
                                #body
                            }
                        }
                    }
                    Fields::Unnamed(fields_unnamed) => {
                        let field_count = fields_unnamed.unnamed.len();
                        let field_idents: Vec<syn::Ident> = (0..field_count)
                            .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                            .collect();
                        quote! {
                            Self::#variant_ident(#( #field_idents ),*) => {
                                #body
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
        Data::Union(_) => unimplemented!("Unions are not supported"), // it doesn't make sense to validate unions as we have no way of knowing which field is active
    }
}

/// Builds one `RodFieldMeta` array entry.
fn meta_entry_tokens(
    name: String,
    group: &Option<LitStr>,
    required: bool,
) -> proc_macro2::TokenStream {
    let group_tokens = match group {
        Some(group) => quote! { Some(#group) },
        None => quote! { None },
    };
    quote! {
        ::rod_validation::RodFieldMeta {
            name: #name,
            group: #group_tokens,
            required: #required,
        },
    }
}

fn is_skip_content(rod_attr: &Option<RodAttr>) -> bool {
    matches!(rod_attr, Some(attr) if matches!(&attr.content, RodAttrContent::Skip(_)))
}

/// Generates `impl RodMetadata for #name` exposing static field metadata.
fn build_meta_impl(ast: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let mut entries: Vec<proc_macro2::TokenStream> = Vec::new();
    match &ast.data {
        Data::Struct(data) => {
            if let Fields::Named(fields_named) = &data.fields {
                for field in &fields_named.named {
                    let (group, rod_attr) = parse_field_rod(field);
                    if is_skip_content(&rod_attr) {
                        continue;
                    }
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    entries.push(meta_entry_tokens(field_name, &group, field_required(&rod_attr)));
                }
            }
        }
        Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                match &variant.fields {
                    Fields::Named(fields_named) => {
                        for field in &fields_named.named {
                            let (group, rod_attr) = parse_field_rod(field);
                            if is_skip_content(&rod_attr) {
                                continue;
                            }
                            let field_name = format!("{}.{}", variant.ident, field.ident.as_ref().unwrap());
                            entries.push(meta_entry_tokens(field_name, &group, field_required(&rod_attr)));
                        }
                    }
                    Fields::Unnamed(fields_unnamed) => {
                        for (idx, field) in fields_unnamed.unnamed.iter().enumerate() {
                            let (group, rod_attr) = parse_field_rod(field);
                            if is_skip_content(&rod_attr) {
                                continue;
                            }
                            let field_name = format!("{}.{}", variant.ident, idx);
                            entries.push(meta_entry_tokens(field_name, &group, field_required(&rod_attr)));
                        }
                    }
                    Fields::Unit => {}
                }
            }
        }
        Data::Union(_) => {}
    }
    quote! {
        impl ::rod_validation::RodMetadata for #name {
            const FIELDS: &'static [::rod_validation::RodFieldMeta] = &[ #( #entries )* ];
        }
    }
}

/// Derives the `RodValidate` trait for a struct.
///
/// Implements validation logic for struct fields annotated with `#[rod(...)]`.
/// Fields without the attribute are required to implement `RodValidate` themselves.
///
/// The following built-in validation types are supported (the keyword must
/// match the field's Rust type):
///
/// - **Strings** — `String`, `str`, `OsString`, `OsStr`, `PathBuf`, `Path`,
///   `Cow`: attributes `length` (exact or range), `format` (requires the
///   `regex` feature), `starts_with`, `ends_with`, `includes`
/// - **Integers** — `i8`–`i128`, `isize`, `u8`–`u128`, `usize`: attributes
///   `size` (exact or range), `sign`, `step`
/// - **Floats** — `f32`, `f64`: attributes `size`, `sign`, `ftype`
/// - **Booleans** — `bool` (no attributes; use `Literal` to assert a value)
/// - **Options** — `Option<T>` wrapping an inner rule, or `Option {}` to
///   require `None`
/// - **Tuples** — `(T1, T2, ...)` with one rule per element
/// - **Iterables** — `Iterable { item: ..., length: ... }` for `Vec` and friends
/// - **Literals** — `Literal { value: ... }` compares against any literal
/// - **Skip** — `#[rod(skip)]` excludes a field from validation
///
/// Any other type is treated as a custom type and must implement
/// `RodValidate`.
/// # Examples
///
/// ```
/// use rod_validation::prelude::*;
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
/// use rod_validation::prelude::*;
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
/// # Custom Validations
/// You can also define custom validations using the `check` attribute. Use a closure to define the validation logic.
/// The closure should take a single argument, which is the field value, and return a boolean indicating whether the validation passed or failed.
/// ```
/// use rod_validation::prelude::*;
/// #[derive(RodValidate)]
/// struct MyEntity {
///    #[rod(
///        String {
///            length: 5..=10,
///        },
///        check = |s| {
///           s.chars().all(|c| c.is_alphanumeric())
///        }
///     )]
///     my_string: String,
/// }
/// let entity = MyEntity {
///     my_string: "Hello123".to_string(),
/// };
/// assert!(entity.validate().is_ok());
/// ```
/// # Validation groups
/// Fields can be tagged with `#[rod(group = "name")]` and validated
/// selectively with `RodValidate::validate_group`.
/// # JSON Schema
/// With the `serde` feature enabled (in both this crate and yours), the derive
/// also generates a `ToJsonSchema` implementation describing the field rules.
/// Generates the code for a single field. `pre_bind` injects the struct-style
/// `let field = &self.field;` binding (empty for enum arms where the match
/// pattern provides the binding). When `guard` is set and the field carries
/// `#[rod(group = "...")]`, the code is wrapped in a `if rod_group == "..."`.
#[proc_macro_error]
#[proc_macro_derive(RodValidate, attributes(rod))]
pub fn derive_rod_validate(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let validations = build_validation_code(&ast, |ret| {
        quote! {
            return Err(#ret);
        }
    }, false);

    let all_validations = build_validation_code(&ast, |ret| {
        quote! {
            errors.push(#ret);
        }
    }, false);

    // Only emit a `validate_group` override when the type actually uses groups.
    let group_override = if has_groups(&ast) {
        let group_validations = build_validation_code(&ast, |ret| {
            quote! {
                errors.push(#ret);
            }
        }, true);
        quote! {
            fn validate_group(&self, rod_group: &str) -> Result<(), RodValidateErrorList> {
                fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateErrorList> {
                    return value.validate_all();
                }
                let mut errors = RodValidateErrorList::new();
                #group_validations
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        }
    } else {
        quote! {}
    };

    let meta_impl = build_meta_impl(&ast);
    let schema_impl = schema::schema_impl_tokens(&ast);
    let async_impl = build_async_checks_impl(&ast);

    quote! {
        impl RodValidate for #name {
            fn validate(&self) -> Result<(), RodValidateError> {
                fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), Vec<RodValidateError>> {
                    let result = value.validate();
                    if result.is_err() {
                        return Err(vec![result.unwrap_err()]);
                    }
                    Ok(())
                }
                #validations
                Ok(())
            }
            fn validate_all(&self) -> Result<(), RodValidateErrorList> {
                fn assert_impl_rod_validate<T: RodValidate>(value: &T) -> Result<(), RodValidateErrorList> {
                    return value.validate_all();
                }
                let mut errors = RodValidateErrorList::new();
                #all_validations
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
            #group_override
        }
        #meta_impl
        #schema_impl
        #async_impl
    }
    .into()
}
