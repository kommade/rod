use proc_macro_error::abort;
use quote::quote;
use syn::{Ident, LitStr, parse::Parse};

use super::user_defined_error;

/// Which typed datetime family a rule applies to. Determines the bound
/// literal grammar and the generated bound constructor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatetimeRuleKind {
    /// `chrono::NaiveDate`, bounds `YYYY-MM-DD`.
    Date,
    /// `chrono::NaiveTime`, bounds `HH:MM[:SS]`.
    Time,
    /// `chrono::NaiveDateTime`, bounds `YYYY-MM-DD` + `T` + `HH:MM[:SS]`.
    NaiveDateTime,
    /// `chrono::DateTime<Tz>` (any timezone), same bounds, interpreted as
    /// UTC (the bound is compared against the value's UTC projection).
    ZonedDateTime,
}

/// One inclusive bound, fully validated at derive time.
pub(crate) struct DatetimeBound {
    /// The literal as written (zero-padded form enforced) - used verbatim
    /// in error messages and schema extensions.
    pub(crate) display: String,
    /// The generated constructor expression resolving to the bound value
    /// (`NaiveDate`, `NaiveTime` or `NaiveDateTime`).
    pub(crate) tokens: proc_macro2::TokenStream,
}

/// Shared `min`/`max` storage for the four datetime rule kinds.
pub(crate) struct DatetimeBounds {
    pub(crate) min: Option<DatetimeBound>,
    pub(crate) max: Option<DatetimeBound>,
    custom_min: Option<LitStr>,
    custom_max: Option<LitStr>,
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn parse_two_digits(s: &str, what: &str, span: proc_macro2::Span) -> u32 {
    if s.len() != 2 || !s.chars().all(|c| c.is_ascii_digit()) {
        abort!(
            span, "Invalid {} `{}` in the bound literal", what, s;
            help = "Numeric components must be zero-padded two digits (00-59 for minutes and seconds)."
        );
    }
    s.parse::<u32>()
        .unwrap_or_else(|_| abort!(span, "Invalid {} `{}`", what, s))
}

/// Validates a `YYYY-MM-DD` literal and returns its components. The
/// calendar is checked at compile time (month range, month lengths,
/// leap-year February).
fn parse_date_part(s: &str, span: proc_macro2::Span) -> (i32, u32, u32) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        abort!(
            span, "Invalid date literal `{}`", s;
            help = "Expected the ISO form `YYYY-MM-DD` (zero-padded), e.g. `\"2026-01-31\"`."
        );
    }
    if parts[0].len() != 4 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
        abort!(
            span, "Invalid year `{}` in the date literal `{}`", parts[0], s;
            help = "Expected a four-digit year, e.g. `\"2026-01-31\"`."
        );
    }
    let year: i32 = parts[0]
        .parse()
        .unwrap_or_else(|_| abort!(span, "Invalid year in `{}`", s));
    let month = parse_two_digits(parts[1], "month", span);
    let day = parse_two_digits(parts[2], "day", span);
    if month < 1 || month > 12 {
        abort!(
            span, "Invalid month {} in the date literal `{}`", month, s;
            help = "Months run 01 through 12."
        );
    }
    let max_day = days_in_month(year, month);
    if day < 1 || day > max_day {
        let month_names = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let name = month_names[(month - 1) as usize];
        abort!(
            span, "Invalid day {} in the date literal `{}`", day, s;
            help = "{} {} has exactly {} days (the calendar is checked at compile time).", name, year, max_day
        );
    }
    (year, month, day)
}

/// Validates an `HH:MM[:SS]` literal (seconds default to 0).
fn parse_time_part(s: &str, span: proc_macro2::Span) -> (u32, u32, u32) {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        abort!(
            span, "Invalid time literal `{}`", s;
            help = "Expected `HH:MM` or `HH:MM:SS`, e.g. `\"07:30\"`."
        );
    }
    if parts[0].len() != 2 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
        abort!(
            span, "Invalid hour `{}` in the time literal `{}`", parts[0], s;
            help = "Hours are zero-padded two digits, 00 through 23."
        );
    }
    let hour: u32 = parts[0]
        .parse()
        .unwrap_or_else(|_| abort!(span, "Invalid hour in `{}`", s));
    if hour > 23 {
        abort!(span, "Invalid hour {} in the time literal `{}`", hour, s; help = "Hours run 00 through 23.");
    }
    let minute = parse_two_digits(parts[1], "minute", span);
    if minute > 59 {
        abort!(span, "Invalid minute {} in the time literal `{}`", minute, s; help = "Minutes run 00 through 59.");
    }
    let second = if parts.len() == 3 {
        let sec = parse_two_digits(parts[2], "second", span);
        if sec > 59 {
            abort!(span, "Invalid second {} in the time literal `{}`", sec, s; help = "Seconds run 00 through 59.");
        }
        sec
    } else {
        0
    };
    (hour, minute, second)
}

fn parse_date_bound(lit: &LitStr) -> DatetimeBound {
    let (y, m, d) = parse_date_part(lit.value().as_str(), lit.span());
    DatetimeBound {
        display: lit.value(),
        tokens: quote! {
            ::rod_validation::chrono::NaiveDate::from_ymd_opt(#y, #m, #d)
                .expect("rod: bound literal validated at compile time")
        },
    }
}

fn parse_time_bound(lit: &LitStr) -> DatetimeBound {
    let (h, m, s) = parse_time_part(lit.value().as_str(), lit.span());
    DatetimeBound {
        display: lit.value(),
        tokens: quote! {
            ::rod_validation::chrono::NaiveTime::from_hms_opt(#h, #m, #s)
                .expect("rod: bound literal validated at compile time")
        },
    }
}

fn parse_datetime_bound(lit: &LitStr) -> DatetimeBound {
    let raw = lit.value();
    let span = lit.span();
    let (date_part, time_part) = match raw.split_once('T') {
        Some(parts) => parts,
        None => abort!(
            span, "Invalid datetime literal `{}`", raw;
            help = "Expected `YYYY-MM-DD` + `T` + `HH:MM[:SS]`, e.g. `\"2026-01-31T08:30\"`."
        ),
    };
    let (y, m, d) = parse_date_part(date_part, span);
    let (h, mi, s) = parse_time_part(time_part, span);
    DatetimeBound {
        display: raw,
        tokens: quote! {
            ::rod_validation::chrono::NaiveDate::from_ymd_opt(#y, #m, #d)
                .expect("rod: bound literal validated at compile time")
                .and_hms_opt(#h, #mi, #s)
                .expect("rod: bound literal validated at compile time")
        },
    }
}

/// Parses the `min`/`max` body of a datetime rule (shared by all four
/// kinds). Requires the `chrono` feature on rod_derive (forwarded by
/// rod_validation's `chrono`) - a clear abort otherwise.
fn parse_bounds(
    input: syn::parse::ParseStream,
    kind: DatetimeRuleKind,
) -> syn::Result<DatetimeBounds> {
    if !cfg!(feature = "chrono") {
        abort!(
            input.span(), "The typed datetime rules require the `chrono` feature";
            help = "Enable `rod_validation/chrono` (which forwards to the derive) and depend on chrono for the field types."
        );
    }
    let opt = super::optional_braced(input)?;
    let inner = match opt {
        Some(inner) => inner,
        None => {
            return Ok(DatetimeBounds {
                min: None,
                max: None,
                custom_min: None,
                custom_max: None,
            });
        }
    };
    let mut min = None;
    let mut max = None;
    let mut custom_min = None;
    let mut custom_max = None;
    let mut message: Option<LitStr> = None;
    while !inner.is_empty() {
        let lookahead = inner.lookahead1();
        if lookahead.peek(Ident) {
            let ident: Ident = inner.parse()?;
            let spelling = ident.to_string();
            if spelling != "min" && spelling != "max" {
                abort!(
                    ident.span(), "Unknown attribute `{}`", spelling;
                    help = "Typed datetime rules support `min` and `max` bounds, e.g. `NaiveDate {{ min: \"2026-01-01\", max: \"2026-12-31\" }}`."
                );
            }
            if spelling == "min" && min.is_some() {
                abort!(ident.span(), "The attribute `min` is used multiple times.");
            }
            if spelling == "max" && max.is_some() {
                abort!(ident.span(), "The attribute `max` is used multiple times.");
            }
            inner.parse::<syn::Token![:]>()?;
            let lit: LitStr = inner.parse()?;
            let bound = match kind {
                DatetimeRuleKind::Date => parse_date_bound(&lit),
                DatetimeRuleKind::Time => parse_time_bound(&lit),
                DatetimeRuleKind::NaiveDateTime | DatetimeRuleKind::ZonedDateTime => {
                    parse_datetime_bound(&lit)
                }
            };
            if spelling == "min" {
                min = Some(bound);
                if let Some(msg) = message.take() {
                    custom_min = Some(msg);
                }
            } else {
                max = Some(bound);
                if let Some(msg) = message.take() {
                    custom_max = Some(msg);
                }
            }
            _ = inner.parse::<syn::Token![,]>();
        } else if lookahead.peek(syn::Token![?]) {
            let _q: syn::Token![?] = inner.parse()?;
            let result: LitStr = inner.parse()?;
            message = Some(result);
        } else {
            abort!(inner.span(), "Expected `min`, `max` or `? \"message\"");
        }
    }
    Ok(DatetimeBounds {
        min,
        max,
        custom_min,
        custom_max,
    })
}

impl DatetimeBounds {
    /// Generates the `min`/`max` checks. `value_expr` is the (already
    /// normalized) value expression to compare - a `NaiveDate`/`NaiveTime`/
    /// `NaiveDateTime` for the naive kinds, and the UTC-naive projection
    /// for zoned fields (bounds are interpreted as UTC there).
    /// `display_value_expr` renders the value for error messages.
    pub(crate) fn code(
        &self,
        field_name: &Ident,
        wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        custom_error: Option<&LitStr>,
        value_expr: proc_macro2::TokenStream,
        display_value_expr: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        let path = field_name.to_string();
        let min = self.min.as_ref().map(|bound| {
            let ret = if let Some(msg) = self.custom_min.as_ref().or(custom_error) {
                user_defined_error(wrap_return, msg)
            } else {
                let display = &bound.display;
                wrap_return(quote! {
                    RodValidateError::Datetime(DatetimeValidation::Min(#path, #display_value_expr, #display.to_string()))
                })
            };
            let tokens = &bound.tokens;
            quote! {
                {
                    let rod_min = #tokens;
                    if #value_expr < rod_min {
                        #ret;
                    }
                }
            }
        });
        let max = self.max.as_ref().map(|bound| {
            let ret = if let Some(msg) = self.custom_max.as_ref().or(custom_error) {
                user_defined_error(wrap_return, msg)
            } else {
                let display = &bound.display;
                wrap_return(quote! {
                    RodValidateError::Datetime(DatetimeValidation::Max(#path, #display_value_expr, #display.to_string()))
                })
            };
            let tokens = &bound.tokens;
            quote! {
                {
                    let rod_max = #tokens;
                    if #value_expr > rod_max {
                        #ret;
                    }
                }
            }
        });
        quote! {
            #min
            #max
        }
    }

    /// Schema extensions for the bounds. JSON Schema has no ordering
    /// keywords for `format: date`-style strings, so the bounds travel as
    /// `x-rod-min`/`x-rod-max`.
    pub(crate) fn schema_exts(&self) -> Vec<(&'static str, String)> {
        let mut exts = Vec::new();
        if let Some(min) = &self.min {
            exts.push(("x-rod-min", min.display.clone()));
        }
        if let Some(max) = &self.max {
            exts.push(("x-rod-max", max.display.clone()));
        }
        exts
    }
}

macro_rules! datetime_content_impls {
    ($ty:ident, $kind:expr) => {
        impl Parse for $ty {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                let bounds = parse_bounds(input, $kind)?;
                Ok($ty { bounds })
            }
        }

        impl $ty {
            pub(crate) const KIND: DatetimeRuleKind = $kind;

            pub(crate) fn schema_exts(&self) -> Vec<(&'static str, String)> {
                self.bounds.schema_exts()
            }
        }

        impl $ty {
            pub(crate) fn get_validations(
                &self,
                field_name: &Ident,
                wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
                _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
            ) -> proc_macro2::TokenStream {
                self.bounds.code(
                    field_name,
                    wrap_return,
                    None,
                    value_expr_for(<$ty>::KIND, field_name),
                    datetime_display_expr(field_name),
                )
            }

            pub(crate) fn get_validations_with_custom_error(
                &self,
                field_name: &Ident,
                wrap_return: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
                _delegate: fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream,
                custom_error: &LitStr,
            ) -> proc_macro2::TokenStream {
                self.bounds.code(
                    field_name,
                    wrap_return,
                    Some(custom_error),
                    value_expr_for(<$ty>::KIND, field_name),
                    datetime_display_expr(field_name),
                )
            }
        }
    };
}

/// Rules for `chrono::NaiveDate` fields: inclusive `min`/`max`.
pub struct RodDateContent {
    pub(crate) bounds: DatetimeBounds,
}
/// Rules for `chrono::NaiveTime` fields: inclusive `min`/`max`.
pub struct RodTimeContent {
    pub(crate) bounds: DatetimeBounds,
}
/// Rules for `chrono::NaiveDateTime` fields (zoneless; bounds compared
/// against the naive value directly).
pub struct RodNaiveDateTimeContent {
    pub(crate) bounds: DatetimeBounds,
}
/// Rules for zoned `chrono::DateTime<Tz>` fields (bounds interpreted as
/// UTC, compared against the value's UTC projection).
pub struct RodZonedDateTimeContent {
    pub(crate) bounds: DatetimeBounds,
}

fn naive_value_expr(field_name: &Ident) -> proc_macro2::TokenStream {
    quote!(*#field_name)
}

fn zoned_value_expr(field_name: &Ident) -> proc_macro2::TokenStream {
    // Zoned fields are compared against the UTC projection so a bound
    // means the same instant regardless of the field's timezone type.
    quote!(#field_name.with_timezone(&::rod_validation::chrono::Utc).naive_utc())
}

fn value_expr_for(kind: DatetimeRuleKind, field_name: &Ident) -> proc_macro2::TokenStream {
    match kind {
        DatetimeRuleKind::Date | DatetimeRuleKind::Time | DatetimeRuleKind::NaiveDateTime => {
            naive_value_expr(field_name)
        }
        DatetimeRuleKind::ZonedDateTime => zoned_value_expr(field_name),
    }
}

fn datetime_display_expr(field_name: &Ident) -> proc_macro2::TokenStream {
    quote!((#field_name).to_string())
}

datetime_content_impls!(RodDateContent, DatetimeRuleKind::Date);
datetime_content_impls!(RodTimeContent, DatetimeRuleKind::Time);
datetime_content_impls!(RodNaiveDateTimeContent, DatetimeRuleKind::NaiveDateTime);
datetime_content_impls!(RodZonedDateTimeContent, DatetimeRuleKind::ZonedDateTime);
