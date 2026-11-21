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
        }

        impl Error for RodValidateError {}

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
                }
            }
        }

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
}