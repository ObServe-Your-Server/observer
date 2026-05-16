use std::fmt::{write, Formatter};
use std::num::TryFromIntError;

#[derive(Debug)]
pub enum MetricsFileFormatError {
    TryFromInt(TryFromIntError)
}


impl std::fmt::Display for MetricsFileFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TryFromInt(e) => write!(f, "try from int error during data length calculation: {e}")
        }
    }
}

impl std::error::Error for MetricsFileFormatError{}

impl From<TryFromIntError> for MetricsFileFormatError {
    fn from(e: TryFromIntError) -> Self {
        Self::TryFromInt(e)
    }
}