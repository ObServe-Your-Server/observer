use std::fmt::Formatter;
use std::num::TryFromIntError;

#[derive(Debug)]
pub enum MetricsFileFormatError {
    TryFromInt(TryFromIntError),
    SerdeEncode(rmp_serde::encode::Error),
    BlockCountError(String),
    HeaderDataTimeError(String),
}

impl std::fmt::Display for MetricsFileFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TryFromInt(e) => write!(
                f,
                "try from int error during data length calculation (maybe content was too long): {e}"
            ),
            Self::SerdeEncode(e) => write!(f, "error during encoding: {e}"),
            Self::BlockCountError(e) => write!(f, "Block Count Error: {e}"),
            Self::HeaderDataTimeError(e) => write!(f, "error: {e}"),
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
