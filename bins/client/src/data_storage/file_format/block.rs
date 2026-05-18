use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::TryFromIntError;
use chrono::{DateTime, Utc};
use prost_types::Type;
use crate::data_storage::file_format::error::MetricsFileFormatError;
// the datasize could be calculated over the data length, but then it can be forgotten.
// i am not sure if this is the right way. for now, it seems right. fix in the future if

// the data first gets stored and during the serialisation or to bytes there the values get
// transformed to le bytes ready to store

// important for me: low endian only applies to multy byte numerical numbers
#[derive(Debug, PartialEq, Eq, Clone)]
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
    pub fn with_data<D>(data: D) -> Result<Block, MetricsFileFormatError>
    where D: Into<Vec<u8>> + 'static
    {
        let mut block = Block::default();
        block.set_data::<D>(data).map_err(MetricsFileFormatError::TryFromInt)?;
        block.checksum = block.compute_checksum();
        Ok(block)
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

    /// this functions sets the data for one data block. Make sure to put in the correct data_type
    /// the type is calculated with a hash
    pub fn set_data<D: Into<Vec<u8>> + 'static>(&mut self, data: D) -> Result<(), TryFromIntError> {
        let data: Vec<u8> = data.into();

        // set size before set data otherwise data is moved
        self.data_size = u32::try_from(data.len())?;
        self.data = data;
        self.data_type = Self::calculate_data_type::<D>();
        self.checksum = self.compute_checksum();
        Ok(())
    }

    fn calculate_data_type<D: 'static>() -> u64 {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<D>().hash(&mut hasher);
        hasher.finish()
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

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }
}

#[cfg(test)]
mod tests {
    use crate::data_storage::file_format::block::Block;

    #[test]
    fn default_init_test(){
        let block = Block::default();
        assert_eq!(block.data_size, 0);
        assert_eq!(block.data_type, 0);
        assert_eq!(block.checksum, block.compute_checksum()); // not optimal
        assert!(block.data.is_empty());
    }

    #[test]
    fn creation_with_data_test(){
        let data = vec![1,2,3,4,5];
        let block = Block::with_data(data.clone()).unwrap();
        assert_eq!(block.data_size, 5);
        assert_eq!(block.data_type, Block::calculate_data_type::<Vec<u8>>());
        assert_eq!(block.data, data);
        assert_eq!(block.checksum, block.compute_checksum());
    }

    #[test]
    fn set_data_later_test(){
        let mut block = Block::default();
        let data = vec![0,1,2,3,4,5,6,7,8,9];
        block.set_data(data.clone()).unwrap();
        assert_eq!(block.data_size, 10);
        assert_eq!(block.data_type, Block::calculate_data_type::<Vec<u8>>());
        assert_eq!(block.data, data);
        assert_eq!(block.checksum, block.compute_checksum());
    }

    #[test]
    fn creation_with_data_too_big_test(){
        // set_data calls u32::try_from(data.len()) — verify that error path is reachable.
        // On 64-bit, usize::MAX > u32::MAX, so try_from must fail.
        let mut block = Block::default();
        let result = block.set_data(vec![0u8; u32::MAX as usize + 1]);
        assert!(result.is_err());
    }
}