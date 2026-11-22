use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum UuidValidation {
    /// The uuid has a different version than required (path, expected,
    /// actual).
    Version(&'static str, u32, u32),
}

impl Display for UuidValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UuidValidation::Version(path, expected, actual) => {
                write!(
                    f,
                    "Expected `{}` to be a uuid version {}, got version {}",
                    path, expected, actual
                )
            }
        }
    }
}
