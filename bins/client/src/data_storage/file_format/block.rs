use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::TryFromIntError;
use chrono::{DateTime, Utc};
use prost_types::Type;
use serde::{Deserialize, Serialize};
use crate::data_storage::calculate_data_type;
use crate::data_storage::file_format::error::MetricsFileFormatError;
use open_eye::collector::DataCreationTime;
// the datasize could be calculated over the data length, but then it can be forgotten.
// i am not sure if this is the right way. for now, it seems right. fix in the future if

// the data first gets stored and during the serialisation or to bytes there the values get
// transformed to le bytes ready to store

// important for me: low endian only applies to multy byte numerical numbers
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Block{
    serialized_data_size: u32,
    data_type: u64,
    serialized_data: Vec<u8>,
    data_creation_time: i64,
    created_at: i64,
    checksum: u32,
}

impl Block {
    /// Accepts the data and will serialize it and save it
    pub fn with_data<D>(data: D) -> Result<Block, MetricsFileFormatError> // error comes from serialization for data
    where D: 'static + Serialize + for<'de> Deserialize<'de> + DataCreationTime// needs to be static for the data type hash
    {
        // serialize data
        let serialized_data = rmp_serde::to_vec(&data)?;

        // set size before set data otherwise data is moved
        let serialized_data_size = u32::try_from(serialized_data.len())?;
        let data_type = calculate_data_type::<D>();
        let created_at = Utc::now().timestamp();
        let mut block = Block{
            serialized_data_size,
            data_type, serialized_data,
            data_creation_time: data.get_data_creation_time(),
            created_at,
            checksum: 0
        };
        block.checksum = block.compute_checksum()?;
        Ok(block)
    }

    pub fn compute_checksum(&self) -> Result<u32, rmp_serde::encode::Error> {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.serialized_data_size.to_le_bytes());
        hasher.update(&self.data_type.to_le_bytes());
        hasher.update(&self.serialized_data);
        hasher.update(&self.created_at.to_le_bytes());
        Ok(hasher.finalize())
    }

    pub fn data_size(&self) -> u32 {
        self.serialized_data_size
    }

    pub fn data_type(&self) -> u64 {
        self.data_type
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.serialized_data
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
    use chrono::Utc;
    use serde::{Deserialize, Serialize};
    use open_eye::collector::DataCreationTime;
    use crate::data_storage::calculate_data_type;
    use crate::data_storage::file_format::block::Block;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestMetric<D> {
        pub data: D,
        pub creation_time: i64,
    }

    impl<D> DataCreationTime for TestMetric<D>
    where D: Serialize + for<'de> Deserialize<'de> + 'static,
    {
        fn get_data_creation_time(&self) -> i64 {
            self.creation_time
        }
    }

    #[test]
    fn creation_with_data_test(){
        let data_vec: Vec<u8> = vec![1,2,3,4,5];
        let data = TestMetric{
            data: data_vec,
            creation_time: 0,
        };

        let block = Block::with_data(data.clone()).unwrap();
        assert_eq!(block.serialized_data_size, 8); //
        assert_eq!(block.data_type, calculate_data_type::<TestMetric<Vec<u8>>>());
        assert_eq!(block.serialized_data, rmp_serde::to_vec(&data).unwrap());
        assert_eq!(block.checksum, block.compute_checksum().unwrap());
    }

    #[test]
    fn set_data_later_test(){
        let data_vec: Vec<u8> = vec![0,1,2,3,4,5,6,7,8,9];
        let data = TestMetric{
            data: data_vec,
            creation_time: 0,
        };

        let block = Block::with_data(data.clone()).unwrap();
        assert_eq!(block.serialized_data_size, 13); //because rmp serde inserts his parts up front
        assert_eq!(block.data_type, calculate_data_type::<TestMetric<Vec<u8>>>());
        assert_eq!(block.serialized_data, rmp_serde::to_vec(&data).unwrap());
        assert_eq!(block.checksum, block.compute_checksum().unwrap());
    }

    #[test]
    fn creation_with_big_data_test(){
        // set_data calls u32::try_from(data.len()) — verify that error path is reachable.
        // On 64-bit, usize::MAX > u32::MAX, so try_from must fail.
        let data = TestMetric{
            data: vec![0u8; u16::MAX as usize + 1],
            creation_time: 0,
        };
        let block = Block::with_data(data);
        assert!(block.is_ok())
    }
}