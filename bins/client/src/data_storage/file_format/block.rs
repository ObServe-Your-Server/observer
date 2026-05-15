use chrono::{DateTime, Utc};

pub struct Block {
    data: Vec<u8>,
    created_at: i64,
    checksum: u32,
}

impl Block {
    pub fn default() -> Self {
        let mut b = Block{
            data: vec![],
            created_at: Utc::now().timestamp(),
            checksum: 0
        };
        
        b.checksum = b.compute_checksum();
        b
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.data);
        hasher.update(&self.created_at.to_le_bytes());
        hasher.finalize()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        bytes.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes
    }
}