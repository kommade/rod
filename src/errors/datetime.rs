use std::fmt::{Display, Formatter};

/// Errors raised by the typed datetime rules (`NaiveDate`, `NaiveTime`,
/// `NaiveDateTime` and `DateTime<Tz>` fields with `min`/`max` bounds).
/// Bounds are inclusive; both values are stored as their Display form.
#[derive(Debug, Clone)]
pub enum DatetimeValidation {
    /// The value is earlier than the `min` bound (third field).
    Min(&'static str, String, String),
    /// The value is later than the `max` bound (third field).
    Max(&'static str, String, String),
}

impl Display for DatetimeValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DatetimeValidation::Min(path, value, bound) => {
                write!(
                    f,
                    "Expected `{}` to be no earlier than {}, got {}",
                    path, bound, value
                )
            }
            DatetimeValidation::Max(path, value, bound) => {
                write!(
                    f,
                    "Expected `{}` to be no later than {}, got {}",
                    path, bound, value
                )
            }
        }
    }
}
