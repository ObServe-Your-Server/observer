use crate::data_storage::file_format::header::Header;

pub mod header;

pub struct MetricsFile{
    header: Header
}