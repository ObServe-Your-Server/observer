use std::fmt::Formatter;
use std::num::TryFromIntError;

#[derive(Debug)]
pub enum MetricsFileFormatError {
    TryFromInt(TryFromIntError),
    SerdeEncode(rmp_serde::encode::Error),
}

impl std::fmt::Display for MetricsFileFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TryFromInt(e) => write!(f, "try from int error during data length calculation: {e}"),
            Self::SerdeEncode(e) => write!(f, "error during encoding: {e}"),
        }
    }
}

impl std::error::Error for MetricsFileFormatError {}

impl From<TryFromIntError> for MetricsFileFormatError {
    fn from(e: TryFromIntError) -> Self {
        Self::TryFromInt(e)
    }
}

impl From<rmp_serde::encode::Error> for MetricsFileFormatError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        Self::SerdeEncode(e)
    }
}