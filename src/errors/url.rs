use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum UrlValidation {
    /// The url uses a different scheme (path, actual, expected).
    Scheme(&'static str, String, String),
    /// The url is not hosted on the required host (path, actual `<none>`
    /// when host-less like mailto:, expected).
    Host(&'static str, String, String),
    /// The url's host does not end in the required suffix (path, actual,
    /// expected suffix).
    HostSuffix(&'static str, String, String),
}

impl Display for UrlValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlValidation::Scheme(path, actual, expected) => {
                write!(
                    f,
                    "Expected `{}` to have scheme {}, got scheme {}",
                    path, expected, actual
                )
            }
            UrlValidation::Host(path, actual, expected) => {
                write!(
                    f,
                    "Expected `{}` to be hosted on {}, got {}",
                    path, expected, actual
                )
            }
            UrlValidation::HostSuffix(path, actual, expected) => {
                write!(
                    f,
                    "Expected `{}` to be hosted under a name ending in {}, got {}",
                    path, expected, actual
                )
            }
        }
    }
}
