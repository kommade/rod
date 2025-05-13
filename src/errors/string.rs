use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum StringValidation {
    Length(String, String),
    Format(String, &'static str),
}

impl Display for StringValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringValidation::Length(s, r) => write!(f, "Expected string length {}, got {}", r, s.len()),
            StringValidation::Format(s, format) => write!(f, "Expected string with format {}, got {}", format, s),
        }
    }
}