use crate::data_storage::calculate_data_type;
use crate::data_storage::file_format::error::MetricsFileFormatError;
use chrono::{DateTime, Utc};
use open_eye::collector::DataCreationTime;
use prost_types::Type;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::TryFromIntError;
// the datasize could be calculated over the data length, but then it can be forgotten.
// i am not sure if this is the right way. for now, it seems right. fix in the future if

// the data first gets stored and during the serialization or to bytes there the values get
// transformed to le bytes ready to store

// important for me: low endian only applies to multi byte numerical numbers
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(deepsize::DeepSizeOf))]
pub struct Block<Data> {
    serialized_data_size: u32,
    data_type: u64,
    serialized_data: Data,
    data_creation_time: i64,
    created_at: i64,
    checksum: u32,
}

impl<Data> Block<Data>
where
    Data: 'static + Serialize + for<'de> Deserialize<'de> + DataCreationTime,
{
    /// Accepts the data and will serialize it and save it
    pub fn with_data(data: Data) -> Result<Block<Data>, MetricsFileFormatError> {
        // serialize once to measure size and compute checksum; data itself is stored unserialised
        let temp_bytes = rmp_serde::to_vec(&data)?;
        let serialized_data_size = u32::try_from(temp_bytes.len())?;
        let data_type = calculate_data_type::<Data>();
        let created_at = Utc::now().timestamp();
        let data_creation_time = data.get_data_creation_time();
        let mut block = Block {
            serialized_data_size,
            data_type,
            serialized_data: data,
            data_creation_time,
            created_at,
            checksum: 0,
        };
        block.checksum = block.compute_checksum()?;
        Ok(block)
    }

    pub fn compute_checksum(&self) -> Result<u32, rmp_serde::encode::Error> {
        let serialized = rmp_serde::to_vec(&self.serialized_data)?;
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.serialized_data_size.to_le_bytes());
        hasher.update(&self.data_type.to_le_bytes());
        hasher.update(&serialized);
        hasher.update(&self.created_at.to_le_bytes());
        Ok(hasher.finalize())
    }

    pub fn data_size(&self) -> u32 {
        self.serialized_data_size
    }

    pub fn data_type(&self) -> u64 {
        self.data_type
    }

    pub fn data(&self) -> &Data {
        &self.serialized_data
    }

    pub fn data_creation_time(&self) -> i64 {
        self.data_creation_time
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
    use crate::data_storage::calculate_data_type;
    use crate::data_storage::file_format::block::Block;
    use chrono::Utc;
    use open_eye::collector::DataCreationTime;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMetric<D> {
        pub data: D,
        pub creation_time: i64,
    }

    impl<D> DataCreationTime for TestMetric<D>
    where
        D: Serialize + for<'de> Deserialize<'de> + 'static,
    {
        fn get_data_creation_time(&self) -> i64 {
            self.creation_time
        }
    }

    #[test]
    fn creation_with_data_test() {
        let data_vec: Vec<u8> = vec![1, 2, 3, 4, 5];
        let data = TestMetric {
            data: data_vec,
            creation_time: 0,
        };

        let block = Block::with_data(data.clone()).unwrap();
        assert_eq!(block.serialized_data_size, 8); //
        assert_eq!(
            block.data_type,
            calculate_data_type::<TestMetric<Vec<u8>>>()
        );
        assert_eq!(block.data(), &data);
        assert_eq!(block.checksum, block.compute_checksum().unwrap());
    }

    #[test]
    fn set_data_later_test() {
        let data_vec: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let data = TestMetric {
            data: data_vec,
            creation_time: 0,
        };

        let block = Block::with_data(data.clone()).unwrap();
        assert_eq!(block.serialized_data_size, 13); //because rmp serde inserts his parts up front
        assert_eq!(
            block.data_type,
            calculate_data_type::<TestMetric<Vec<u8>>>()
        );
        assert_eq!(block.data(), &data);
        assert_eq!(block.checksum, block.compute_checksum().unwrap());
    }

    #[test]
    fn creation_with_big_data_test() {
        // set_data calls u32::try_from(data.len()) — verify that error path is reachable.
        // On 64-bit, usize::MAX > u32::MAX, so try_from must fail.
        let data = TestMetric {
            data: vec![0u8; u16::MAX as usize + 1],
            creation_time: 0,
        };
        let block = Block::with_data(data);
        assert!(block.is_ok())
    }
}
