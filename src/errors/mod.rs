use std::{error::Error, fmt::{Display, Formatter}};

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

        #[derive(Debug, Clone)]
        pub enum RodValidateError {
            $(
                $tuple_name($mod_name::$type_name),
            )*
        }

        impl Error for RodValidateError {}

        impl Display for RodValidateError {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        RodValidateError::$tuple_name(validation) => 
                            write!(f, "Error validating {}: {}", stringify!($mod_name), validation),
                    )*
                }
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