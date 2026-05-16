use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::TryFromIntError;
use std::ptr::hash;
use chrono::{DateTime, Utc};
use prost_types::Type;
use crate::data_storage::file_format::error::MetricsFileFormatError;
// the datasize could be calculated over the data length, but then it can be forgotten.
// i am not sure if this is the right way. for now, it seems right. fix in the future if

// the data first gets stored and during the serialisation or to bytes there the values get
// transformed to le bytes ready to store

// important for me: low endian only applies to multy byte numerical numbers
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
    /// Accepts the data and will turn it into le bytes to store in the block
    pub fn with_data<'a, D>(data: D) -> Result<Block, MetricsFileFormatError>
    where D: Into<Vec<u8>> + TryFrom<u32> + 'static
    {
        let mut block = Block::default();
        match block.set_data::<D>(data) {
            Ok(_) => Ok(block),
            Err(e) => Err(MetricsFileFormatError::TryFromInt(e))
        }
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

    pub fn calculate_data_type<D: 'static>() -> u64 {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<D>().hash(&mut hasher);
        hasher.finish()
    }

    /// this functions sets the data for one data block. Make sure to put in the correct data_type
    /// the type is calculated with a hash
    pub fn set_data<D: Into<Vec<u8>> + 'static>(&mut self, data: D) -> Result<(), TryFromIntError> {
        let data = data.into();

        // set size before set data otherwise data is moved
        self.data_size = u32::try_from(data.len())?;
        self.data = data;
        self.data_type = Self::calculate_data_type::<D>();
        Ok(())
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }
}