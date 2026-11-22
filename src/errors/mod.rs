use std::{ops::Index, error::Error, fmt::{Display, Formatter}};

macro_rules! rod_validation_types {
    (
        $(
            $mod_name:ident, $tuple_name:ident, $type_name:ident
        ),* $(,)?
    ) => {
        $(
            mod $mod_name;
            pub use $mod_name::$type_name;
        )*

        /// An error that can occur during validation.
        /// This is a sum type of all possible validation errors.
        /// It also includes a variant for custom validation checks that fail.
        /// This is used in the `validate` method of the `RodValidate` trait.
        #[derive(Debug, Clone)]
        pub enum RodValidateError {
            $(
                $tuple_name($mod_name::$type_name),
            )*
            CheckFailed(&'static str),
            UserDefined(String),
            /// An error raised while validating a nested field that itself
            /// implements `RodValidate`. Keeps the field name so the full
            /// path (`outer.inner`) can be reconstructed; nestings produce
            /// nested `Nested` layers.
            Nested(&'static str, Box<RodValidateError>),
        }

        impl RodValidateError {
            /// The path of field names leading to this error, outermost
            /// first (e.g. `["person", "address", "city"]`). Empty for
            /// top-level errors.
            pub fn path(&self) -> Vec<&str> {
                match self {
                    RodValidateError::Nested(field, inner) => {
                        let mut path: Vec<&str> = vec![field];
                        path.extend(inner.path());
                        path
                    }
                    _ => Vec::new(),
                }
            }

            /// The top-level field this error belongs to: the first segment
            /// of [`path`](Self::path). `None` for errors raised at the root
            /// of the validated type (per-value checks, container-shape
            /// errors, `UserDefined` messages without field context).
            pub fn top_level_field(&self) -> Option<&'static str> {
                match self {
                    RodValidateError::Nested(field, _) => Some(field),
                    _ => None,
                }
            }
        }

        impl Display for RodValidateError {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        RodValidateError::$tuple_name(validation) => 
                            write!(f, "{}", validation),
                    )*
                    RodValidateError::CheckFailed(path) => 
                        write!(f, "Custom validation check failed for `{}`", path),
                    RodValidateError::UserDefined(msg) => 
                        write!(f, "{}", msg),
                    RodValidateError::Nested(_field, _inner) => {
                        // The nesting chain renders as a dotted path prefix.
                        let mut path: Vec<&str> = Vec::new();
                        let mut current = self;
                        while let RodValidateError::Nested(field, next) = current {
                            path.push(field);
                            current = next;
                        }
                        write!(f, "{}: {}", path.join("."), current)
                    }
                }
            }
        }

        impl Error for RodValidateError {}

        /// A list of validation errors.
        /// This is used in the `validate_all` method of the `RodValidate` trait
        #[derive(Debug, Clone)]
        pub struct RodValidateErrorList(Vec<RodValidateError>);

        impl Default for RodValidateErrorList {
            fn default() -> Self {
                RodValidateErrorList::new()
            }
        }

        impl RodValidateErrorList {
            /// Creates a new empty error list.
            pub fn new() -> Self {
                RodValidateErrorList(Vec::new())
            }
            /// Pushes an error to the list.
            pub fn push(&mut self, error: RodValidateError) {
                self.0.push(error);
            }
            /// Returns true if the list is empty.
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
            /// Returns the number of errors in the list.
            pub fn len(&self) -> usize {
                self.0.len()
            }
            /// Returns an iterator over the errors without consuming the list.
            pub fn iter(&self) -> std::slice::Iter<'_, RodValidateError> {
                self.0.iter()
            }
            /// Borrow every error as a slice (iteration order preserved).
            pub fn as_slice(&self) -> &[RodValidateError] {
                &self.0
            }

            /// Each error as `(dotted path, rendered message)`, in insertion
            /// order. Root-level errors (no path) get an empty path string.
            pub fn flatten(&self) -> Vec<(String, String)> {
                self.0
                    .iter()
                    .map(|e| (e.path().join("."), e.to_string()))
                    .collect()
            }

            /// Errors grouped by top-level field: field name -> messages,
            /// sorted by field name. The shape UI layers map straight onto
            /// per-field error displays. Root-level errors are omitted;
            /// collect them with [`root_errors`](Self::root_errors).
            pub fn field_errors(&self) -> std::collections::BTreeMap<String, Vec<String>> {
                let mut map = std::collections::BTreeMap::new();
                for e in &self.0 {
                    if let Some(field) = e.top_level_field() {
                        map.entry(field.to_string())
                            .or_insert_with(Vec::new)
                            .push(e.to_string());
                    }
                }
                map
            }

            /// Errors raised at the root of the validated type — they carry
            /// no field path (custom per-value checks, container-shape
            /// errors, `UserDefined` messages without field context).
            pub fn root_errors(&self) -> Vec<&RodValidateError> {
                self.0.iter().filter(|e| e.path().is_empty()).collect()
            }

            /// All errors whose top-level field is `name`, insertion order.
            pub fn field(&self, name: &str) -> Vec<&RodValidateError> {
                self.0
                    .iter()
                    .filter(|e| e.top_level_field() == Some(name))
                    .collect()
            }
        }

        impl Index<usize> for RodValidateErrorList {
            type Output = RodValidateError;

            fn index(&self, index: usize) -> &Self::Output {
                &self.0[index]
            }
        }


        impl IntoIterator for RodValidateErrorList {
            type Item = RodValidateError;
            type IntoIter = std::vec::IntoIter<RodValidateError>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl Error for RodValidateErrorList {}

        impl Display for RodValidateErrorList {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                if self.0.is_empty() {
                    return write!(f, "No validation errors");
                }
                write!(f, "Got {} errors while validating: [\n", self.0.len())?;
                for (i, error) in self.0.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",\n")?;
                    }
                    write!(f, "    {}", error)?;
                }
                write!(f, "\n]")
            }
        }
    }
}

rod_validation_types! {
    string, String, StringValidation,
    integer, Integer, IntegerValidation,
    literal, Literal, LiteralValidation,
    option, Option, OptionValidation,
    float, Float, FloatValidation,
    iterable, Iterable, IterableValidation,
    datetime, Datetime, DatetimeValidation,
    uuid, Uuid, UuidValidation,
    url, Url, UrlValidation,
}