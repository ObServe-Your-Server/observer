use std::iter::Map;
use std::slice::Iter;
use serde::{Deserialize, Serialize};
use crate::data_storage::file_format::block::Block;
use crate::data_storage::file_format::error::MetricsFileFormatError;
use crate::data_storage::file_format::header::Header;
use crate::data_storage::serializer::Serializer;

pub mod header;
mod block;
mod error;
pub mod metrics_file;