pub struct BmfBlock {

}

pub enum DeserializationError {}

pub trait BmfSave {

}

pub trait BmfLoad {
    // there is a trait sized but then at compile time the result size needs to be known which is
    // not possible when there is dynamic data in the block
    fn deserialize(b: &[u8]) -> Result<Box<Self>, DeserializationError>;
}