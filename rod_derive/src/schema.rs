//! Compile-time JSON Schema generation from `#[rod(...)]` validation rules.
//!
//! The derive macro renders the schema to a JSON string literal at compile
//! time; the generated code parses it into a `serde_json::Value` at runtime.
//! The generated impl block is gated behind `#[cfg(feature = "serde")]` of the
//! deriving crate (which conventionally enables `rod_validation/serde`).

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DataEnum, DataStruct, DeriveInput, ExprRange, Fields, LitInt, RangeLimits};

use crate::{parse_field_description, parse_field_rod, RodAttrContent};
use crate::types::{LengthOrSize, NumberSign};

/// A tiny compile-time JSON representation.
pub(crate) enum Json {
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn s(v: impl Into<String>) -> Json {
        Json::Str(v.into())
    }
    fn n_i(v: i128) -> Json {
        Json::Num(v.to_string())
    }
    fn n_f(v: f64) -> Json {
        // Render floats losslessly enough for schema bounds.
        if v == v.trunc() && v.abs() < 1e15 {
            Json::Num(format!("{}", v as i128))
        } else {
            Json::Num(format!("{}", v))
        }
    }
    fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn render(&self) -> String {
        match self {
            Json::Bool(b) => b.to_string(),
            Json::Num(n) => n.clone(),
            Json::Str(s) => format!("\"{}\"", json_escape(s)),
            Json::Arr(items) => {
                let inner: Vec<String> = items.iter().map(|i| i.render()).collect();
                format!("[{}]", inner.join(", "))
            }
            Json::Obj(pairs) => {
                // Later keys must not override earlier ones (e.g. a field
                // declaring both `size` and `sign: Nonnegative` emits two
                // `minimum` constraints; the tighter size one wins).
                let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
                let inner: Vec<String> = pairs
                    .iter()
                    .filter(|(k, _)| seen.insert(k.as_str()))
                    .map(|(k, v)| format!("\"{}\": {}", json_escape(k), v.render()))
                    .collect();
                format!("{{{}}}", inner.join(", "))
            }
        }
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Strips Rust integer/float type suffixes (`42u8`, `1.5f64`) and digit
/// separators, returning the numeric text (or `None` if not a plain number).
fn clean_number(text: &str) -> Option<String> {
    let mut s = text.trim().to_string();
    let negative = s.starts_with('-');
    if negative {
        s.remove(0);
    }
    s = s.chars().filter(|c| *c != '_').collect();
    while s
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphabetic())
    {
        s.pop();
    }
    if s.is_empty() {
        return None;
    }
    // Keep non-exponent forms tidy: "2.0" stays, "2." becomes "2.0".
    if s.ends_with('.') {
        s.push('0');
    }
    s.parse::<f64>().ok()?;
    let v = if negative { format!("-{}", s) } else { s };
    Some(v)
}

fn expr_i128(expr: Option<&Box<syn::Expr>>) -> Option<i128> {
    let expr = expr?;
    let ts = expr.to_token_stream();
    if let Ok(lit) = syn::parse2::<LitInt>(ts.clone()) {
        return lit.base10_parse().ok();
    }
    let s = ts.to_string();
    let (neg, digits) = match s.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, s.as_str()),
    };
    digits
        .parse::<i128>()
        .ok()
        .map(|v| if neg { -v } else { v })
}

fn expr_f64(expr: Option<&Box<syn::Expr>>) -> Option<f64> {
    let text = clean_number(&expr?.to_token_stream().to_string())?;
    text.parse::<f64>().ok()
}

/// Converts a Rust string literal body (already unquoted) to its actual
/// content, undoing the common Rust escapes.
fn rust_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('\'') => out.push('\''),
            Some('"') => out.push('"'),
            Some('u') => {
                // \u{XXXX}
                if chars.next() == Some('{') {
                    let mut hex = String::new();
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        hex.push(c);
                    }
                    if let Ok(v) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(v) {
                            out.push(ch);
                        }
                    }
                }
            }
            Some('x') => {
                let mut hex = String::new();
                for _ in 0..2 {
                    if let Some(c) = chars.next() {
                        hex.push(c);
                    }
                }
                if let Ok(v) = u8::from_str_radix(&hex, 16) {
                    out.push(v as char);
                }
            }
            _ => out.push('\\'),
        }
    }
    out
}

/// Extracts the JSON value + schema type from a Rust literal token text
/// (e.g. `"hello"`, `42u8`, `true`, `'a'`, `2.5f64`).
fn literal_to_json(lit: &syn::PatLit) -> (String, &'static str) {
    let text = lit.lit.to_token_stream().to_string();
    let trimmed = text.trim();

    // Raw or plain string.
    if trimmed.starts_with("r#\"") || trimmed.starts_with("r\"") {
        let inner = trimmed
            .strip_prefix("r#\"")
            .and_then(|s| s.strip_suffix("\"#"))
            .or_else(|| trimmed.strip_prefix("r\"").and_then(|s| s.strip_suffix('"')));
        if let Some(inner) = inner {
            return (inner.to_string(), "string");
        }
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        return (rust_unescape(inner), "string");
    }
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        let content = rust_unescape(inner);
        // Char literal content may still keep escape backslashes for symbols;
        // unescape handles the common ones.
        let content = content.trim_start_matches('\\');
        return (content.to_string(), "string");
    }
    if trimmed == "true" || trimmed == "false" {
        return (trimmed.to_string(), "boolean");
    }
    if let Some(num) = clean_number(trimmed) {
        return (num, "number");
    }
    // Fallback: treat it as an opaque string.
    (trimmed.to_string(), "string")
}

fn literal_schema(content: &crate::types::RodLiteralContent) -> Option<Json> {
    // `any: [...]` form -> JSON Schema `enum` (typed per literal kind).
    if !content.any().is_empty() {
        let values: Vec<Json> = content
            .any()
            .iter()
            .map(|lit| {
                let (value, ty) = literal_to_json(lit);
                match ty {
                    "boolean" => Json::Bool(value == "true"),
                    "number" => Json::Num(value),
                    _ => Json::Str(value),
                }
            })
            .collect();
        return Some(Json::obj(vec![("enum", Json::Arr(values))]));
    }
    let lit = content.value()?;
    let (value, ty) = literal_to_json(lit);
    // Emit `const` with the proper JSON type, not as a string.
    let const_json = match ty {
        "boolean" => Json::Bool(value == "true"),
        "number" => Json::Num(value),
        _ => Json::Str(value),
    };
    Some(Json::obj(vec![("type", Json::s(ty)), ("const", const_json)]))
}

fn string_schema(content: &crate::types::RodStringContent) -> Option<Json> {
    let mut fields = vec![("type", Json::s("string"))];
    if let Some(length) = content.length() {
        match length {
            LengthOrSize::Exact(exact) => {
                if let Ok(n) = exact.base10_parse::<i128>() {
                    fields.push(("minLength", Json::n_i(n)));
                    fields.push(("maxLength", Json::n_i(n)));
                }
            }
            LengthOrSize::Range(range) => {
                let (start, end, inclusive) = range_bounds_i128(range);
                if let Some(min) = start {
                    fields.push(("minLength", Json::n_i(min)));
                }
                if let Some(max) = end {
                    if inclusive {
                        fields.push(("maxLength", Json::n_i(max)));
                    } else {
                        // Lengths are integers, so `..end` means `max <= end - 1`.
                        fields.push(("maxLength", Json::n_i(max - 1)));
                    }
                }
            }
        }
    }
    #[cfg(feature = "regex")]
    if let Some(format) = content.format() {
        use crate::types::StringFormat;
        match format {
            StringFormat::Regex(lit) => fields.push(("pattern", Json::s(lit.value()))),
            StringFormat::Email => fields.push(("format", Json::s("email"))),
            StringFormat::Url => fields.push(("format", Json::s("uri"))),
            StringFormat::Uuid => fields.push(("format", Json::s("uuid"))),
            StringFormat::Ipv4 => fields.push(("format", Json::s("ipv4"))),
            StringFormat::Ipv6 => fields.push(("format", Json::s("ipv6"))),
            StringFormat::DateTime => fields.push(("format", Json::s("date-time"))),
            StringFormat::Date => fields.push(("format", Json::s("date"))),
            StringFormat::Time => fields.push(("format", Json::s("time"))),
            StringFormat::Duration => fields.push(("format", Json::s("duration"))),
            StringFormat::Base64 => fields.push(("format", Json::s("base64"))),
            StringFormat::Ulid => fields.push(("format", Json::s("ulid"))),
            StringFormat::Cuid => fields.push(("format", Json::s("cuid"))),
            StringFormat::Cuid2 => fields.push(("format", Json::s("cuid2"))),
        }
    }
    if let Some(starts) = content.starts_with() {
        fields.push(("x-rod-starts-with", Json::s(starts.value())));
    }
    if let Some(ends) = content.ends_with() {
        fields.push(("x-rod-ends-with", Json::s(ends.value())));
    }
    if let Some(includes) = content.includes() {
        fields.push(("x-rod-includes", Json::s(includes.value())));
    }
    Some(Json::obj(fields))
}

fn sign_schema(sign: &NumberSign) -> Vec<(&'static str, Json)> {
    match sign {
        NumberSign::Positive => vec![("exclusiveMinimum", Json::n_i(0))],
        NumberSign::Negative => vec![("exclusiveMaximum", Json::n_i(0))],
        NumberSign::Nonnegative => vec![("minimum", Json::n_i(0))],
        NumberSign::Nonpositive => vec![("maximum", Json::n_i(0))],
    }
}

fn integer_schema(content: &crate::types::RodIntegerContent) -> Option<Json> {
    let mut fields = vec![("type", Json::s("integer"))];
    if let Some(size) = content.size() {
        match size {
            LengthOrSize::Exact(exact) => {
                if let Ok(n) = exact.base10_parse::<i128>() {
                    fields.push(("minimum", Json::n_i(n)));
                    fields.push(("maximum", Json::n_i(n)));
                }
            }
            LengthOrSize::Range(range) => {
                let (start, end, inclusive) = range_bounds_i128(range);
                if let Some(min) = start {
                    fields.push(("minimum", Json::n_i(min)));
                }
                if let Some(max) = end {
                    if inclusive {
                        fields.push(("maximum", Json::n_i(max)));
                    } else {
                        fields.push(("exclusiveMaximum", Json::n_i(max)));
                    }
                }
            }
        }
    }
    if let Some(sign) = content.sign() {
        fields.extend(sign_schema(sign));
    }
    if let Some(step) = content.step() {
        if let Ok(n) = step.base10_parse::<i128>() {
            fields.push(("multipleOf", Json::n_i(n)));
        }
    }
    Some(Json::obj(fields))
}

fn float_schema(content: &crate::types::RodFloatContent) -> Option<Json> {
    let mut fields = vec![("type", Json::s("number"))];
    if let Some(size) = content.size() {
        match size {
            // `LengthOrSize::Exact` only parses integer literals, so the
            // validation engine accepts e.g. `size: 6` for floats.
            LengthOrSize::Exact(exact) => {
                if let Ok(n) = exact.base10_parse::<i128>() {
                    fields.push(("minimum", Json::n_i(n)));
                    fields.push(("maximum", Json::n_i(n)));
                }
            }
            LengthOrSize::Range(range) => {
                let (start, end, inclusive) = range_bounds_f64(range);
                if let Some(min) = start {
                    fields.push(("minimum", Json::n_f(min)));
                }
                if let Some(max) = end {
                    if inclusive {
                        fields.push(("maximum", Json::n_f(max)));
                    } else {
                        fields.push(("exclusiveMaximum", Json::n_f(max)));
                    }
                }
            }
        }
    }
    if let Some(sign) = content.sign() {
        fields.extend(sign_schema(sign));
    }
    if let Some(step) = content.step() {
        let n = match step {
            syn::Lit::Float(l) => l.to_token_stream().to_string().parse::<f64>().ok(),
            syn::Lit::Int(l) => l.base10_parse::<f64>().ok(),
            _ => None,
        };
        if let Some(n) = n {
            fields.push(("multipleOf", Json::n_f(n)));
        }
    }
    Some(Json::obj(fields))
}

fn iterable_schema(content: &crate::types::RodIterableContent) -> Option<Json> {
    let mut fields = vec![("type", Json::s("array"))];
    // `item: Skip` leaves items unconstrained (Skip schemas to a blank).
    let item_schema = content_schema(&content.item.content).unwrap_or(Json::Bool(true));
    fields.push(("items", item_schema));
    if let Some(length) = content.length() {
        match length {
            LengthOrSize::Exact(exact) => {
                if let Ok(n) = exact.base10_parse::<i128>() {
                    fields.push(("minItems", Json::n_i(n)));
                    fields.push(("maxItems", Json::n_i(n)));
                }
            }
            LengthOrSize::Range(range) => {
                let (start, end, inclusive) = range_bounds_i128(range);
                if let Some(min) = start {
                    fields.push(("minItems", Json::n_i(min)));
                }
                if let Some(max) = end {
                    if inclusive {
                        fields.push(("maxItems", Json::n_i(max)));
                    } else {
                        fields.push(("maxItems", Json::n_i(max - 1)));
                    }
                }
            }
        }
    }
    if content.distinct {
        fields.push(("uniqueItems", Json::Bool(true)));
    }
    Some(Json::obj(fields))
}

/// Schema for a `Map` field: a JSON object whose properties (the values)
/// are constrained by the `value` rule. The `length` becomes the property
/// count bounds. Key rules are runtime-only: JSON Schema cannot express
/// per-key constraints beyond string patterns, so they are not exported.
fn map_schema(content: &crate::types::RodMapContent) -> Option<Json> {
    let mut fields = vec![("type", Json::s("object"))];
    // `value: Skip` leaves values unconstrained (blank schema).
    let value_schema = content
        .value
        .as_ref()
        .and_then(|v| content_schema(&v.content))
        .unwrap_or(Json::Bool(true));
    fields.push(("additionalProperties", value_schema));
    if let Some(length) = content.length() {
        match length {
            LengthOrSize::Exact(exact) => {
                if let Ok(n) = exact.base10_parse::<i128>() {
                    fields.push(("minProperties", Json::n_i(n)));
                    fields.push(("maxProperties", Json::n_i(n)));
                }
            }
            LengthOrSize::Range(range) => {
                let (start, end, inclusive) = range_bounds_i128(range);
                if let Some(min) = start {
                    fields.push(("minProperties", Json::n_i(min)));
                }
                if let Some(max) = end {
                    if inclusive {
                        fields.push(("maxProperties", Json::n_i(max)));
                    } else {
                        fields.push(("maxProperties", Json::n_i(max - 1)));
                    }
                }
            }
        }
    }
    Some(Json::obj(fields))
}

fn tuple_schema(content: &crate::types::RodTupleContent) -> Option<Json> {
    let prefix: Vec<Json> = content
        .fields
        .iter()
        .map(|f| content_schema(&f.content).unwrap_or(Json::Bool(true)))
        .collect();
    Some(Json::obj(vec![
        ("type", Json::s("array")),
        ("prefixItems", Json::Arr(prefix)),
    ]))
}

fn content_schema(content: &RodAttrContent) -> Option<Json> {
    match content {
        RodAttrContent::Skip(_) => None,
        RodAttrContent::Boolean(_) => Some(Json::obj(vec![("type", Json::s("boolean"))])),
        RodAttrContent::Custom(_) => Some(Json::obj(vec![("type", Json::s("object"))])),
        RodAttrContent::Literal(l) => literal_schema(l),
        RodAttrContent::String(s) => string_schema(s),
        RodAttrContent::Integer(i) => integer_schema(i),
        RodAttrContent::Float(f) => float_schema(f),
        RodAttrContent::Option(o) => match &o.inner {
            // `Option {}` means "must be None".
            None => Some(Json::obj(vec![("type", Json::s("null"))])),
            Some(inner) => Some(
                content_schema(&inner.content)
                    .unwrap_or(Json::obj(vec![("type", Json::s("null"))])),
            ),
        },
        RodAttrContent::Iterable(i) => iterable_schema(i),
        RodAttrContent::Map(m) => map_schema(m),
        RodAttrContent::Tuple(t) => tuple_schema(t),
    }
}

/// Schema hints for unattributed third-party datetime fields (the derive's
/// `chrono` feature, forwarded by rod_validation's `chrono` feature). The
/// types validate trivially via blanket impls; the schema only annotates the
/// wire format chrono's serde produces.
#[cfg(feature = "chrono")]
fn third_party_datetime_schema(ty: &syn::Type) -> Option<Json> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let format = match path.path.segments.last()?.ident.to_string().as_str() {
        "NaiveDate" => Some("date"),
        "NaiveTime" => Some("time"),
        // Zoned datetimes serialize as RFC 3339, so the standard keyword is
        // exactly right.
        "DateTime" => Some("date-time"),
        // NaiveDateTime is zoneless and therefore NOT valid RFC 3339 - a
        // strict `date-time` validator would reject the real data. The
        // x-rod extension keeps the hint without overpromising.
        "NaiveDateTime" => {
            return Some(Json::obj(vec![
                ("type", Json::s("string")),
                ("x-rod-format", Json::s("naive-date-time")),
            ]));
        }
        // Bare `Duration` is ambiguous (chrono's TimeDelta vs
        // std::time::Duration, which serializes as {secs, nanos}), so it is
        // deliberately left unannotated.
        _ => None,
    };
    format.map(|format| {
        Json::obj(vec![("type", Json::s("string")), ("format", Json::s(format))])
    })
}

/// Schema for a field, plus whether the field is required. `None` means the
/// field is skipped and should not appear in the schema at all.
fn field_schema(field: &syn::Field) -> Option<(Option<Json>, bool)> {
    let (_, rod_attr) = parse_field_rod(field);
    let description = parse_field_description(field);
    if rod_attr.is_none() && matches!(&field.ty, syn::Type::BareFn(_)) {
        // Callback fields (form handlers etc.) carry behavior, not data:
        // they are excluded from the schema exactly like `Skip` fields.
        return None;
    }
    let attach = |schema: Option<Json>| -> Option<Json> {
        match (schema, description.as_ref()) {
            (Some(Json::Obj(mut pairs)), Some(desc)) => {
                if !pairs.iter().any(|(k, _)| k == "description") {
                    pairs.push(("description".to_string(), Json::Str(desc.value())));
                }
                Some(Json::Obj(pairs))
            }
            (schema, _) => schema,
        }
    };
    match &rod_attr {
        // Fields without a `rod` attribute must implement `RodValidate`
        // themselves; we cannot introspect their rules, so emit a generic object.
        None => {
            #[cfg(feature = "chrono")]
            if let Some(schema) = third_party_datetime_schema(&field.ty) {
                return Some((attach(Some(schema)), true));
            }
            Some((
                attach(Some(Json::obj(vec![("type", Json::s("object"))]))),
                true,
            ))
        }
        Some(attr) => {
            let schema = content_schema(&attr.content);
            let required = !matches!(
                &attr.content,
                RodAttrContent::Skip(_) | RodAttrContent::Option(_)
            );
            Some((attach(schema), required))
        }
    }
}

/// Builds `{ "type": "object", "properties": {...}, "required": [...] }` from
/// named fields (used for both structs and named enum variants).
fn object_from_fields<'a, I: IntoIterator<Item = &'a syn::Field>>(fields: I) -> Json {
    let mut properties: Vec<(String, Json)> = Vec::new();
    let mut required: Vec<Json> = Vec::new();
    for field in fields {
        let name = field.ident.as_ref().map(|i| i.to_string());
        let Some(name) = name else { continue };
        if let Some((schema, req)) = field_schema(field) {
            if let Some(schema) = schema {
                properties.push((name.clone(), schema));
            }
            if req {
                required.push(Json::Str(name));
            }
        }
    }
    let mut pairs: Vec<(&str, Json)> = vec![("type", Json::s("object"))];
    pairs.push(("properties", Json::Obj(properties)));
    if !required.is_empty() {
        pairs.push(("required", Json::Arr(required)));
    }
    Json::obj(pairs)
}

/// Schema for an unattributed custom-type field (nested `RodValidate`).
fn inner_field_schema(field: &syn::Field) -> Option<Json> {
    match field_schema(field) {
        Some((schema, _required)) => schema,
        None => Some(Json::obj(vec![("type", Json::s("object"))])),
    }
}

fn struct_schema(data: &DataStruct) -> Json {
    match &data.fields {
        Fields::Named(named) => object_from_fields(&named.named),
        // Newtype structs validate their single inner field; the schema is
        // the inner field's rule (same idea as Rust's transparent types).
        Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => inner_field_schema(
            unnamed.unnamed.first().unwrap(),
        )
        .unwrap_or(Json::obj(vec![("type", Json::s("object"))])),
        // Multi-field tuple structs are rejected at codegen; unit structs
        // accept anything.
        Fields::Unnamed(_) => Json::obj(vec![("type", Json::s("object"))]),
        Fields::Unit => Json::Bool(true),
    }
}

fn enum_schema(data: &DataEnum) -> Json {
    let variants: Vec<Json> = data
        .variants
        .iter()
        .map(|variant| match &variant.fields {
            Fields::Named(named) => object_from_fields(&named.named),
            Fields::Unnamed(unnamed) => {
                let prefix: Vec<Json> = unnamed
                    .unnamed
                    .iter()
                    .map(|field| {
                        let (_, rod_attr) = parse_field_rod(field);
                        match rod_attr {
                            None => Json::obj(vec![("type", Json::s("object"))]),
                            Some(attr) => {
                                content_schema(&attr.content).unwrap_or(Json::Bool(true))
                            }
                        }
                    })
                    .collect();
                Json::obj(vec![
                    ("type", Json::s("array")),
                    ("prefixItems", Json::Arr(prefix)),
                ])
            }
            Fields::Unit => Json::obj(vec![("type", Json::s("null"))]),
        })
        .collect();
    Json::obj(vec![("oneOf", Json::Arr(variants))])
}

fn range_bounds_i128(range: &ExprRange) -> (Option<i128>, Option<i128>, bool) {
    let start = expr_i128(range.start.as_ref());
    let end = expr_i128(range.end.as_ref());
    let inclusive = matches!(&range.limits, RangeLimits::Closed(_));
    (start, end, inclusive)
}

fn range_bounds_f64(range: &ExprRange) -> (Option<f64>, Option<f64>, bool) {
    let start = expr_f64(range.start.as_ref());
    let end = expr_f64(range.end.as_ref());
    let inclusive = matches!(&range.limits, RangeLimits::Closed(_));
    (start, end, inclusive)
}

/// Generates the `#[cfg(feature = "serde")] impl ToJsonSchema` block.
/// Returns `None` for types we cannot describe (unions).
pub(crate) fn schema_impl_tokens(ast: &DeriveInput) -> Option<TokenStream> {
    let name = &ast.ident;
    let schema = match &ast.data {
        Data::Struct(data) => struct_schema(data),
        Data::Enum(data) => enum_schema(data),
        Data::Union(_) => return None,
    };
    // Lift the schema body (type/properties/required or oneOf) to the
    // document root and add the $schema key.
    let Json::Obj(body) = schema else { return None };
    let mut pairs: Vec<(String, Json)> = Vec::with_capacity(body.len() + 1);
    pairs.push((
        "$schema".to_string(),
        Json::s("https://json-schema.org/draft/2020-12/schema"),
    ));
    pairs.extend(body);
    let rendered = Json::Obj(pairs).render();
    Some(quote! {
        #[cfg(feature = "serde")]
        impl ::rod_validation::ToJsonSchema for #name {
            fn to_json_schema() -> ::rod_validation::__serde_json::Value {
                ::rod_validation::__serde_json::from_str::<::rod_validation::__serde_json::Value>(#rendered)
                    .expect("rod: generated JSON Schema is invalid")
            }
        }
    })
}
