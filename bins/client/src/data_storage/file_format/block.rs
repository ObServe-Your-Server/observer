use std::num::TryFromIntError;
use std::ptr::hash;
use chrono::{DateTime, Utc};

// the datasize could be calculated over the data length, but then it can be forgotten.
// i am not sure if this is the right way. for now, it seems right. fix in the future if
pub struct Block {
    data_size: u32,
    data_type: u64,
    data: Vec<u8>,
    created_at: i64,
    checksum: u32,
}

impl Block {
    pub fn default() -> Self {
        let mut b = Block{
            data_size: 0,
            data_type: 0,
            data: vec![],
            created_at: Utc::now().timestamp(),
            checksum: 0
        };
        
        b.checksum = b.compute_checksum();
        b
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.data_size.to_le_bytes());
        hasher.update(&self.data_type.to_le_bytes());
        hasher.update(&self.data);
        hasher.update(&self.created_at.to_le_bytes());
        hasher.finalize()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.data_size.to_le_bytes());
        bytes.extend_from_slice(&self.data_type.to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    pub fn data_size(&self) -> u32 {
        self.data_size
    }

    pub fn data_type(&self) -> u64 {
        self.data_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn set_data(&mut self, data: Vec<u8>, data_type: u64) -> Result<(), TryFromIntError>{
        // set size before set data otherwise data is moved
        self.data_size = u32::try_from(data.len())?;
        self.data = data;
        self.data_type = data_type;
        Ok(())
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }
}