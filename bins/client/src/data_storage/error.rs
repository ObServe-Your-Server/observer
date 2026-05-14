#[derive(Debug)]
pub enum BinStoreError {
    InvalidExtension,
    Io(std::io::Error),
    Encode(rmp_serde::encode::Error),
    Decode(rmp_serde::decode::Error),
}

impl std::fmt::Display for BinStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidExtension => write!(f, "file must have a .obsr extension"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Encode(e) => write!(f, "encode error: {e}"),
            Self::Decode(e) => write!(f, "decode error: {e}"),
        }
    }
}

impl std::error::Error for BinStoreError {}

impl From<std::io::Error> for BinStoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<rmp_serde::encode::Error> for BinStoreError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        Self::Encode(e)
    }
}

impl From<rmp_serde::decode::Error> for BinStoreError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        Self::Decode(e)
    }
}
