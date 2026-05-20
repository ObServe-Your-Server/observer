use std::thread::sleep;
use serde::{Deserialize, Serialize};
use crate::data_storage::file_format::error::MetricsFileFormatError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header{
    magic: [u8; 8],
    version: u8,
    pad: [u8; 4],
    block_count: u32,
    pub first_metric_timestamp: Option<i64>,
    pub last_metric_timestamp: Option<i64>,
    checksum: u32
}

impl Header{
    pub fn default() -> Self {
        let mut h = Header{
            magic: [b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'],
            version: 1,
            pad: [0,0,0,0],
            block_count: 0,
            checksum: 0,
            first_metric_timestamp: None, //TODO
            last_metric_timestamp: None //TODO
        };
        h.checksum = h.compute_checksum();
        h
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.magic);
        hasher.update(&[self.version]);
        hasher.update(&self.pad);
        hasher.update(&self.block_count.to_le_bytes());

        self.first_metric_timestamp.inspect(|e| hasher.update(&e.to_le_bytes()));
        self.last_metric_timestamp.inspect(|e| hasher.update(&e.to_le_bytes()));
        hasher.finalize()
    }
    
    fn update_checksum(&mut self) -> u32 {
        self.checksum = self.compute_checksum();
        self.checksum
    }

    pub fn increment_block_count(&mut self, data_creation_time: i64) -> Result<(), MetricsFileFormatError> {
        self.block_count = self.block_count.checked_add(1).ok_or(MetricsFileFormatError::ToManyBlocks(format!("You try to save too many elements in one file. There are already {}", self.block_count)))?;

        self.first_metric_timestamp = Some(match self.first_metric_timestamp {
            None => data_creation_time,
            Some(existing) => existing.min(data_creation_time),
        });
        self.last_metric_timestamp = Some(match self.last_metric_timestamp {
            None => data_creation_time,
            Some(existing) => existing.max(data_creation_time),
        });
        
        self.update_checksum();
        Ok(())
    }

    pub fn magic(&self) -> [u8;8] {
        self.magic
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn pad(&self) -> [u8;4] {
        self.pad
    }

    pub fn block_count(&self) -> u32 {
        self.block_count
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn verify(&self) -> bool {
        self.checksum == self.compute_checksum()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::data_storage::file_format::header::Header;

    #[test]
    fn default_test(){
        let header = Header::default();
        let header_magic = vec![b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'];

        assert_eq!(header_magic, header.magic);
        assert_eq!(header.version, 1);
        assert_eq!(header.pad, [0,0,0,0]);
        assert_eq!(header.block_count, 0);
        assert_eq!(header.first_metric_timestamp, None);
        assert_eq!(header.last_metric_timestamp, None);
        assert_eq!(header.checksum, 371124639);
        assert_eq!(header.checksum, header.compute_checksum());
    }
}