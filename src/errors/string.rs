use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum StringValidation {
    Length(&'static str, String, String),
    Format(&'static str, String, &'static str),
    StartsWith(&'static str, String, String),
    EndsWith(&'static str, String, String),
    Includes(&'static str, String, String),
}

impl Display for StringValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringValidation::Length(path, s, r) => write!(f, "Expected `{}` to have length {}, got {}", path, r, s.len()),
            StringValidation::Format(path, s, format) => write!(f, "Expected `{}` to have format {}, got {}", path, format, s),
            StringValidation::StartsWith(path, s, prefix) => write!(f, "Expected `{}` to start with {}, got {}", path, prefix, s),
            StringValidation::EndsWith(path, s, suffix) => write!(f, "Expected `{}` to end with {}, got {}", path, suffix, s),
            StringValidation::Includes(path, s, substring) => write!(f, "Expected `{}` to include {}, got {}", path, substring, s),
        }
    }
}