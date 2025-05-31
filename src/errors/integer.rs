use std::fmt::{Display, Formatter};

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

#[derive(Debug, Clone)]
pub enum IntegerValidation {
    Size(&'static str, Integer, String),
    Sign(&'static str, Integer, &'static str),
    Step(&'static str, Integer, Integer),
}

impl Display for IntegerValidation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerValidation::Size(path, int, size) => write!(f, "Expected `{}` to be an integer {}, got {}", path, size, int),
            IntegerValidation::Sign(path, int, sign) => write!(f, "Expected `{}` to be an integer with sign {}, got {}", path, sign, int),
            IntegerValidation::Step(path, int, step) => write!(f, "Expected `{}` to be an integer with step {}, got {}", path, step, int),
        }
    }
}