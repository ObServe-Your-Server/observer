use crate::data_storage::file_format::block::Block;
use crate::data_storage::file_format::header::Header;

pub mod header;
mod block;
mod error;

pub struct MetricsFileFormat {
    header: Header,
    blocks: Option<Vec<Block>>,
    checksum: u32,
}

impl MetricsFileFormat {
    pub fn default() -> Self {
        let mut mf = MetricsFileFormat {
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

    pub fn add_block() {

    }
}

#[cfg(test)]
mod tests {
    use crate::data_storage::file_format::MetricsFileFormat;

    #[test]
    fn test_default_generation(){
        let bare_bone = MetricsFileFormat::default();

        let header_magic = vec![b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'];
        assert_eq!(header_magic, bare_bone.header.magic);

        assert_eq!(bare_bone.checksum, 558161692);
        assert!(bare_bone.blocks.is_none());
    }
}