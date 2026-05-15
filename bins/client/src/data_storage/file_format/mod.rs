use crate::data_storage::file_format::block::Block;
use crate::data_storage::file_format::header::Header;

pub mod header;
mod block;

pub struct MetricsFile{
    header: Header,
    blocks: Option<Vec<Block>>,
    checksum: u32,
}

impl MetricsFile {
    pub fn default() -> Self {
        let mut mf = MetricsFile{
            header: Header::default(),
            blocks: None,
            checksum: 0,
        };
        mf.checksum = mf.compute_checksum();
        mf
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        // hash the header bytes
        hasher.update(&self.header.to_bytes());
        // hash each block's bytes
        if let Some(blocks) = &self.blocks {
            for block in blocks {
                hasher.update(&block.to_bytes());
            }
        }
        hasher.finalize()
    }
}