use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum StringValidation {
    Length(String, String),
    Format(String, &'static str),
    StartsWith(String, String),
    EndsWith(String, String),
    Includes(String, String),
}

impl Display for StringValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringValidation::Length(s, r) => write!(f, "Expected string length {}, got {}", r, s.len()),
            StringValidation::Format(s, format) => write!(f, "Expected string with format {}, got {}", format, s),
            StringValidation::StartsWith(s, prefix) => write!(f, "Expected string to start with {}, got {}", prefix, s),
            StringValidation::EndsWith(s, suffix) => write!(f, "Expected string to end with {}, got {}", suffix, s),
            StringValidation::Includes(s, substring) => write!(f, "Expected string to include {}, got {}", substring, s),
        }
    }
}