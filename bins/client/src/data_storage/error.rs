use std::fmt::Formatter;
use std::io;

#[derive(Debug)]
pub enum DataStorageError {
    EmptyBasePath(String),
    NoDataForGivenDataId,
    FileIo(io::Error),
    RmpSerdeEncodeError(rmp_serde::encode::Error),
}

impl std::fmt::Display for DataStorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self { 
            Self::EmptyBasePath(e) => write!(f, "empty base path for metrics file {e}"),
            Self::NoDataForGivenDataId => write!(f, "for the given data id was no data found to save"),
            Self::FileIo(e) => write!(f, "{e}"),
            Self::RmpSerdeEncodeError(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DataStorageError {}

impl From<io::Error> for DataStorageError{
    fn from(value: io::Error) -> Self {
        DataStorageError::FileIo(value)
    }
}

impl From<rmp_serde::encode::Error> for DataStorageError {
    fn from(value: rmp_serde::encode::Error) -> Self {
        DataStorageError::RmpSerdeEncodeError(value)
    }
}