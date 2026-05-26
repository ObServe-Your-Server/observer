use std::cmp::min;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use open_eye::collector::DataCreationTime;
use crate::data_storage::file_format::block::Block;
use crate::data_storage::file_format::error::MetricsFileFormatError;
use crate::data_storage::file_format::header::Header;
use crate::data_storage::serializer::Serializer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsFile {
    header: Header,
    blocks: Option<Vec<Block>>,
    checksum: u32,
}

impl MetricsFile {
    pub fn default() -> Result<Self, rmp_serde::encode::Error> {
        let mut mf = MetricsFile {
            header: Header::default(),
            blocks: None,
            checksum: 0,
        };
        mf.checksum = mf.compute_checksum()?;
        Ok(mf)
    }

    pub fn with_data<D>(data: Vec<D>) -> Result<Self, MetricsFileFormatError>
    where D: Serialize + for<'de> Deserialize<'de> + 'static + DataCreationTime
    {
        let mut file = MetricsFile::default()?;
        data.into_iter().map(|d| file.add_data_block(d)).collect::<Result<(), _>>()?;
        Ok(file)
    }

    pub fn add_data_block<D>(&mut self, data: D) -> Result<(), MetricsFileFormatError>
    where D: Serialize + for<'de> Deserialize<'de> + 'static + DataCreationTime
    {
        self.header.increment_block_count(data.get_data_creation_time())?;

        self.blocks = match self.blocks.take() {
            None => {
                // there is no data so set both the same
                self.header.first_metric_timestamp = Some(data.get_data_creation_time());
                self.header.last_metric_timestamp = Some(data.get_data_creation_time());
                Some(vec![Block::with_data(data)?])
            },
            Some(mut blocks) => {
                self.header.last_metric_timestamp = Some(data.get_data_creation_time());
                blocks.push(Block::with_data(data)?);
                Some(blocks)
            }
        };

        let first_metric_timestamp = self.blocks.as_ref().ok_or(MetricsFileFormatError::HeaderDataTimeError("no data entry found (should not happen)".to_string()))?
            .iter()
            .map(|b| b.data_creation_time())
            .min()
            .ok_or(MetricsFileFormatError::HeaderDataTimeError("no first element found to set in the block header for first data block.".to_string()))?;

        self.header.first_metric_timestamp = Some(first_metric_timestamp);

        let last_metric_timestamp = self.blocks.as_ref().ok_or(MetricsFileFormatError::HeaderDataTimeError("no data entry found (should not happen)".to_string()))?
            .iter()
            .map(|b| b.data_creation_time())
            .max()
            .ok_or(MetricsFileFormatError::HeaderDataTimeError("no last element found to set in the block header for first data block.".to_string()))?;

        self.header.last_metric_timestamp = Some(last_metric_timestamp);
        Ok(())
    }

    /// Computes the checksum by first serializing the datablocks.
    /// Then it all adds it to the hasher and generates the u32.
    /// The error comes from serializing
    pub fn compute_checksum(&self) -> Result<u32, rmp_serde::encode::Error> {
        let mut hasher = crc32fast::Hasher::new();
        // hash the header bytes
        hasher.update(&Serializer::serialize(&self.header)?);
        // hash each block's bytes
        if let Some(blocks) = &self.blocks {
            for block in blocks {
                hasher.update(&Serializer::serialize(&block)?);
            }
        }
        Ok(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde::{Deserialize, Serialize};
    use open_eye::collector::cpu::collector::CpuStats;
    use open_eye::collector::DataCreationTime;
    use crate::data_storage;
    use crate::data_storage::file_format::header::Header;
    use crate::data_storage::file_format::metrics_file::MetricsFile;
    use crate::data_storage::serializer::Serializer;

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
    fn default_generation_test(){
        let bare_bone = MetricsFile::default().unwrap(); //any type can be used just a test

        let header_magic = vec![b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'];
        assert_eq!(header_magic, bare_bone.header.magic());
        assert_eq!(bare_bone.header.checksum().clone(), bare_bone.header.compute_checksum());
        assert_eq!(bare_bone.header.block_count().clone(), 0);
        assert_eq!(bare_bone.header.pad().clone(), [0,0,0,0]);

        assert_eq!(bare_bone.header.first_metric_timestamp, None);
        assert_eq!(bare_bone.header.last_metric_timestamp, None);

        assert_eq!(bare_bone.checksum, 3852221886);
        assert!(bare_bone.blocks.is_none());
    }

    #[test]
    fn generate_with_data_test(){
        let data = vec![vec![1u8,2,3,4,5], vec![6,7,8,9,0], vec![11,12,13,14,15]];

        let vec_data: Vec<TestMetric<Vec<u8>>> = data.into_iter().enumerate().map(|(i, e)| TestMetric {
            data: e,
            creation_time: i as i64,
        }).collect();

        let file = MetricsFile::with_data(vec_data.clone()).unwrap();

        let blocks = file.blocks.as_ref().unwrap();
        for (i, entry) in vec_data.iter().enumerate() {
            let block_data: TestMetric<Vec<u8>> = rmp_serde::from_slice(blocks[i].data()).unwrap();
            assert_eq!(block_data.data, entry.data);
            assert_eq!(block_data.creation_time, entry.creation_time);
        }

        assert_eq!(blocks[0].data_type(), blocks[1].data_type());
        assert_eq!(blocks[1].data_type(), blocks[2].data_type());

        assert_eq!(file.header.first_metric_timestamp.unwrap(), 0);
        assert_eq!(file.header.last_metric_timestamp.unwrap(), 2);

        println!("file: {:#?}", file);
    }

    #[test]
    fn default_test(){
        let file = MetricsFile::default().unwrap();
        let default_header = Header::default();
        assert_eq!(file.header, default_header);
        assert_eq!(file.blocks, None);
        assert_eq!(file.checksum, 3852221886);
    }

   /* #[test]
    fn serialization_deserialization_test(){
        let cpu_data = CpuStats::get_current_stats();
        let mut vec = Vec::new();
        vec.push(cpu_data);
        let file = MetricsFile::with_data(vec).unwrap();
        println!("plain file: {:#?}", file);

        let serialized_file = Serializer::serialize(&file).unwrap();
        println!("serialized file: {:#?}", serialized_file);

        let deserialized_file = Serializer::deserialize::<MetricsFile>(&serialized_file).unwrap();
        println!("deserialized file: {:#?}", file);

        assert_eq!(file, deserialized_file);
    }*/
}