use std::iter::Map;
use std::slice::Iter;
use crate::data_storage::file_format::block::Block;
use crate::data_storage::file_format::error::MetricsFileFormatError;
use crate::data_storage::file_format::header::Header;
use crate::data_storage::serializer::Serializer;

pub mod header;
mod block;
mod error;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    where D: Into<Vec<u8>> + 'static
    {
        let mut file = MetricsFile::default()?;
        data.into_iter().map(|d| file.add_data_block(d)).collect::<Result<(), _>>()?;
        Ok(file)
    }

    pub fn add_data_block<D>(&mut self, data: D) -> Result<(), MetricsFileFormatError>
    where D: Into<Vec<u8>> + 'static
    {
        self.header.increment_block_count();
        self.blocks = match self.blocks.take() {
            None => Some(vec![Block::with_data(data)?]),
            Some(mut blocks) => {
                blocks.push(Block::with_data(data)?);
                Some(blocks)
            }
        };
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
    use crate::data_storage::file_format::MetricsFile;

    #[test]
    fn default_generation_test(){
        let bare_bone = MetricsFile::default().unwrap();

        let header_magic = vec![b'O', b'B', b'S', b'E', b'R', b'V', b'E', b'R'];
        assert_eq!(header_magic, bare_bone.header.magic());
        assert_eq!(bare_bone.header.checksum().clone(), bare_bone.header.compute_checksum());
        assert_eq!(bare_bone.header.block_count().clone(), 0);
        assert_eq!(bare_bone.header.pad().clone(), [0,0,0,0]);

        assert_eq!(bare_bone.header.first_metric_timestamp(), None);
        assert_eq!(bare_bone.header.last_metric_timestamp(), None);

        assert_eq!(bare_bone.checksum, 3852221886);
        assert!(bare_bone.blocks.is_none());
    }

    #[test]
    fn generate_with_data_test(){
        let data = vec![vec![1,2,3,4,5], vec![6,7,8,9,0], vec![11,12,13,14,15]];
        let file = MetricsFile::with_data(data.clone()).unwrap();

        let first_block_data = file.blocks.as_ref().unwrap().first().unwrap().data();
        let second_block_data = file.blocks.as_ref().unwrap().get(1).unwrap().data();
        let last_block_data = file.blocks.as_ref().unwrap().last().unwrap().data();

        assert_eq!(data.first().unwrap(), first_block_data);
        assert_eq!(data.get(1).unwrap(), second_block_data);
        assert_eq!(data.last().unwrap(), last_block_data);

        let blocks = file.blocks.as_ref().unwrap();
        assert_eq!(blocks[0].data_type(), blocks[1].data_type());
        assert_eq!(blocks[1].data_type(), blocks[2].data_type());
    }
}