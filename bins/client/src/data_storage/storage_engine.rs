use std::any::TypeId;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;
use std::env::current_dir;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::iter::Once;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use chrono::Utc;
use erased_serde::Serialize as ErasedSerialize;
use crate::data_storage::calculate_data_type;
use crate::data_storage::error::DataStorageError;

pub trait DataBlockEntry: ErasedSerialize + Debug {}

// implement serialization again
erased_serde::serialize_trait_object!(DataBlockEntry);

#[derive(Debug)]
pub struct ChannelEntry{
    data_block_entry: Box<dyn DataBlockEntry>,
    inserted_at: chrono::DateTime<Utc>
}

const FILE_EXTENSION: &str = "obs";
#[derive(Debug)]
pub struct StorageEngine{
    storage_channels: HashMap<u64, Vec<ChannelEntry>>,
    save_to_file_interval: OnceCell<u16>,
    base_folder: OnceCell<PathBuf>
}

impl StorageEngine {

    /// Start the storage engine
    pub fn default() -> Result<Self, io::Error>{
        Ok(StorageEngine{
            storage_channels: HashMap::default(),
            save_to_file_interval: OnceCell::from(30),
            base_folder: OnceCell::from(current_dir()?)
        })
    }

    // static because to calculate the datatype all data needs to be owned.
    // (so not shared reference)
    pub fn add_data<D: DataBlockEntry + 'static>(&mut self, data: D) {
        let key = calculate_data_type::<D>();
        let entry = ChannelEntry {
            data_block_entry: Box::new(data),
            inserted_at: Utc::now(),
        };
        self.storage_channels.entry(key).or_default().push(entry);
    }

    pub fn remove_channel(&mut self, data_type_key: &u64) -> Option<Vec<ChannelEntry>> {
        self.storage_channels.remove(data_type_key)
    }

    pub fn get_channel_elements(&mut self, data_type_key: &u64) -> Option<&Vec<ChannelEntry>> {
        self.storage_channels.get(data_type_key)
    }

    pub fn save_to_file(&self, data_type_key: &u64) -> Result<(),DataStorageError>{
        let mut file_path = self.base_folder.get().ok_or(DataStorageError::EmptyBasePath(String::from("Base path was not initialised for Storage Engine")))?.clone();
        let current_time = Utc::now().timestamp();
        file_path.push(format!("{}.{}.{}", data_type_key, current_time, FILE_EXTENSION));

        let file = File::create_new(file_path);
        // TODO: hier weitermachen
        todo!()
    }

    pub fn channel_key_for_data_type<D>() -> u64
    where D: 'static
    {
        calculate_data_type::<D>()
    }
    
}

#[cfg(test)]
mod tests {
    use std::cell::OnceCell;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use open_eye::collector::cpu::collector::CpuStats;
    use open_eye::collector::memory::collector::MemoryStats;
    use crate::data_storage::storage_engine::StorageEngine;

    #[test]
    fn default_engine_test(){
        let storage_engine = StorageEngine::default().unwrap();
        println!("{:#?}", storage_engine);
    }

    #[test]
    fn once_cell_cannot_be_set_twice() {
        let storage_engine = StorageEngine::default().unwrap();
        assert!(storage_engine.save_to_file_interval.set(99).is_err());
        assert!(storage_engine.base_folder.set(PathBuf::from(".")).is_err());
    }

    #[test]
    fn add_one_data_to_one_channel(){
        let mut storage_engine = StorageEngine::default().unwrap();

        let cpu_data = CpuStats::get_current_stats();
        storage_engine.add_data(cpu_data);

        println!("{:#?}", storage_engine);
    }

    #[test]
    fn add_multiple_data_to_one_channel(){
        let mut storage_engine = StorageEngine::default().unwrap();

        let cpu_data = CpuStats::get_current_stats();
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data);

        let vec_length = storage_engine.get_channel_elements(&StorageEngine::channel_key_for_data_type::<CpuStats>()).unwrap().len();
        assert_eq!(vec_length, 5);

        println!("Vec length: {}", vec_length);
        println!("{:#?}", storage_engine);
    }

    #[test]
    fn add_multiple_data_to_multiple_channel(){
        let mut storage_engine = StorageEngine::default().unwrap();

        let cpu_data = CpuStats::get_current_stats();
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data);

        let memory_data = MemoryStats::get_current_stats();
        storage_engine.add_data(memory_data.clone());
        storage_engine.add_data(memory_data.clone());
        storage_engine.add_data(memory_data.clone());
        storage_engine.add_data(memory_data.clone());

        let vec_length_cpu = storage_engine.get_channel_elements(&StorageEngine::channel_key_for_data_type::<CpuStats>()).unwrap().len();
        let vec_length_memory = storage_engine.get_channel_elements(&StorageEngine::channel_key_for_data_type::<MemoryStats>()).unwrap().len();

        assert_eq!(vec_length_cpu, 5);
        assert_eq!(vec_length_memory, 4);


        println!("Vec length CPU: {}", vec_length_cpu);
        println!("Vec length Memory: {}", vec_length_memory);
        println!("{:#?}", storage_engine);
    }
}