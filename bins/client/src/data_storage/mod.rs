pub mod serializer;
mod file_format;
mod storage_engine;
mod entries;
mod error;

use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};

pub fn calculate_data_type<D: 'static>() -> u64 {
    let mut hasher = DefaultHasher::new();
    TypeId::of::<D>().hash(&mut hasher);
    hasher.finish()
}