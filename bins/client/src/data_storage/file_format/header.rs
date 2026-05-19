use std::thread::sleep;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header{
    magic: [u8; 8],
    version: u8,
    pad: [u8; 4],
    block_count: u32,
    first_metric_timestamp: Option<u64>,
    last_metric_timestamp: Option<u64>,
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
            first_metric_timestamp: None,
            last_metric_timestamp: None
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

    pub fn increment_block_count(&mut self) -> u32 {
        self.block_count += 1;
        self.update_checksum();
        self.checksum()
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

    pub fn first_metric_timestamp(&self) -> Option<u64> {
        self.first_metric_timestamp
    }

    pub fn last_metric_timestamp(&self) -> Option<u64> {
        self.last_metric_timestamp
    }
    
    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn verify(&self) -> bool {
        self.checksum == self.compute_checksum()
    }
}