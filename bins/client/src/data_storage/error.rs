use std::fmt::Formatter;

#[derive(Debug)]
pub enum DataStorageError {
    EmptyBasePath(String)
}

impl std::fmt::Display for DataStorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self { 
            Self::EmptyBasePath(e) => write!(f, "empty base path for metrics file {e}")
        }
    }
}

impl std::error::Error for DataStorageError {}