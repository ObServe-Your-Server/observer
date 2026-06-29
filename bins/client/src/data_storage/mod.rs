pub mod serializer;
pub mod file_format;
pub mod storage_engine;
mod entries;
mod error;

use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};

pub fn calculate_data_type<D: 'static>() -> u64 {
    let mut hasher = DefaultHasher::new();
    TypeId::of::<D>().hash(&mut hasher);
    hasher.finish()
}
