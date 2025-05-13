use std::{error::Error, fmt::{Display, Formatter}};

macro_rules! impl_from_integer {
    ($name:ident, $integer:ty) => {
        impl From<$integer> for Integer {
            fn from(integer: $integer) -> Self {
                Integer::$name(integer.into())
            }
        }
    };
}

#[derive(Debug, Clone)]
pub enum Integer {
    Negative(i128),
    Positive(u128),
}

impl_from_integer!(Negative, i8);
impl_from_integer!(Positive, u8);
impl_from_integer!(Negative, i16);
impl_from_integer!(Positive, u16);
impl_from_integer!(Negative, i32);
impl_from_integer!(Positive, u32);
impl_from_integer!(Negative, i64);
impl_from_integer!(Positive, u64);
impl_from_integer!(Negative, i128);
impl_from_integer!(Positive, u128);
impl From<isize> for Integer {
    fn from(integer: isize) -> Self {
        Integer::Negative(integer as i128)
    }
}
impl From<usize> for Integer {
    fn from(integer: usize) -> Self {
        Integer::Positive(integer as u128)
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer::Negative(i) => write!(f, "{}", i),
            Integer::Positive(i) => write!(f, "{}", i),
        }
    }
}

mod string;
pub use string::StringValidation;
mod integer;
pub use integer::IntegerValidation;
mod literal;
pub use literal::LiteralValidation;
mod option;
pub use option::OptionValidation;

#[derive(Debug, Clone)]
pub enum RodValidateError {
    String(string::StringValidation),
    Integer(integer::IntegerValidation),
    Literal(literal::LiteralValidation),
    Option(option::OptionValidation),
}

impl Error for RodValidateError {}


impl Display for RodValidateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RodValidateError::String(validation) => write!(f, "Error validating string: {}", validation),
            RodValidateError::Integer(validation) => write!(f, "Error validating integer: {}", validation),
            RodValidateError::Literal(validation) => write!(f, "Error validating literal: {}", validation),
            RodValidateError::Option(validation) => write!(f, "Error validating option: {}", validation),
        }
    }
}