use crate::data_storage::calculate_data_type;
use crate::data_storage::error::DataStorageError;
use crate::data_storage::file_format::metrics_file::MetricsFile;
use crate::data_storage::serializer::Serializer;
use chrono::Utc;
use open_eye::collector::DataCreationTime;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::env::current_dir;
use std::fmt::Debug;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

const FILE_EXTENSION: &str = "obs";

pub struct StorageEngine {
    storage_channels: HashMap<u64, Box<dyn Any>>,
    save_to_file_interval: OnceCell<u16>,
    base_folder: OnceCell<PathBuf>,
}

impl Debug for StorageEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageEngine")
            .field(
                "channel_keys",
                &self.storage_channels.keys().collect::<Vec<_>>(),
            )
            .field("save_to_file_interval", &self.save_to_file_interval)
            .field("base_folder", &self.base_folder)
            .finish()
    }
}

impl StorageEngine {
    pub fn default() -> Result<Self, io::Error> {
        Ok(StorageEngine {
            storage_channels: HashMap::default(),
            save_to_file_interval: OnceCell::from(30),
            base_folder: OnceCell::from(current_dir()?),
        })
    }

    pub fn with_base_folder(path: PathBuf) -> Self {
        StorageEngine {
            storage_channels: HashMap::default(),
            save_to_file_interval: OnceCell::from(30),
            base_folder: OnceCell::from(path),
        }
    }

    pub fn add_data<D>(&mut self, data: D)
    where
        D: 'static + Serialize + for<'de> Deserialize<'de> + DataCreationTime,
    {
        let key = calculate_data_type::<D>();
        self.storage_channels
            .entry(key)
            .or_insert_with(|| Box::new(Vec::<D>::new()))
            .downcast_mut::<Vec<D>>()
            .unwrap()
            .push(data);
    }

    pub fn remove_channel<D: 'static>(&mut self) -> Option<Vec<D>> {
        self.storage_channels
            .remove(&calculate_data_type::<D>())
            .and_then(|b| b.downcast::<Vec<D>>().ok())
            .map(|b| *b)
    }

    pub fn get_channel_elements<D: 'static>(&self) -> Option<&Vec<D>> {
        self.storage_channels
            .get(&calculate_data_type::<D>())
            .and_then(|b| b.downcast_ref::<Vec<D>>())
    }

    pub fn save_to_file<D>(&mut self) -> Result<(), DataStorageError>
    where
        D: 'static + Serialize + for<'de> Deserialize<'de> + DataCreationTime,
    {
        let key = calculate_data_type::<D>();
        let entries: Vec<D> = self
            .storage_channels
            .remove(&key)
            .ok_or(DataStorageError::NoDataForGivenDataId)
            .and_then(|b| {
                // the downcast here is okay because I know what we saved in at the add_data function
                b.downcast::<Vec<D>>()
                    .map_err(|_| DataStorageError::NoDataForGivenDataId)
            })
            .map(|b| *b)?;

        let mut file_path = self
            .base_folder
            .get()
            .ok_or(DataStorageError::EmptyBasePath(String::from(
                "Base path was not initialised for Storage Engine",
            )))?
            .clone();
        let current_time = Utc::now().timestamp();
        file_path.push(format!("{}.{}.{}", key, current_time, FILE_EXTENSION));

        let bytes = MetricsFile::<D>::with_data(entries)?.to_bytes()?;
        let mut file = File::create_new(file_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    pub fn channel_key_for_data_type<D: 'static>() -> u64 {
        calculate_data_type::<D>()
    }
}

#[cfg(test)]
mod tests {
    use crate::data_storage::storage_engine::StorageEngine;
    use open_eye::collector::cpu::collector::CpuStats;
    use open_eye::collector::memory::collector::MemoryStats;
    use std::path::PathBuf;

    #[test]
    fn default_engine_test() {
        let storage_engine = StorageEngine::default().unwrap();
        println!("{:#?}", storage_engine);
    }

    #[test]
    fn once_cell_cannot_be_set_twice_test() {
        let storage_engine = StorageEngine::default().unwrap();
        assert!(storage_engine.save_to_file_interval.set(99).is_err());
        assert!(storage_engine.base_folder.set(PathBuf::from(".")).is_err());
    }

    #[test]
    fn add_one_data_to_one_channel_test() {
        let mut storage_engine = StorageEngine::default().unwrap();

        let cpu_data = CpuStats::get_current_stats();
        storage_engine.add_data(cpu_data);

        println!("{:#?}", storage_engine);
    }

    #[test]
    fn add_multiple_data_to_one_channel_test() {
        let mut storage_engine = StorageEngine::default().unwrap();

        let cpu_data = CpuStats::get_current_stats();
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data);

        let vec_length = storage_engine
            .get_channel_elements::<CpuStats>()
            .unwrap()
            .len();
        assert_eq!(vec_length, 5);

        println!("Vec length: {}", vec_length);
        println!("{:#?}", storage_engine);
    }

    #[test]
    fn add_multiple_data_to_multiple_channel_test() {
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

        let vec_length_cpu = storage_engine
            .get_channel_elements::<CpuStats>()
            .unwrap()
            .len();
        let vec_length_memory = storage_engine
            .get_channel_elements::<MemoryStats>()
            .unwrap()
            .len();

        assert_eq!(vec_length_cpu, 5);
        assert_eq!(vec_length_memory, 4);

        println!("Vec length CPU: {}", vec_length_cpu);
        println!("Vec length Memory: {}", vec_length_memory);
        println!("{:#?}", storage_engine);
    }

    #[test]
    fn add_lots_of_data_to_multiple_channel_test() {
        let mut storage_engine = StorageEngine::default().unwrap();

        let cpu_data = CpuStats::get_current_stats();
        for _i in 0..100 {
            storage_engine.add_data(cpu_data.clone());
        }

        let memory_data = MemoryStats::get_current_stats();
        for _i in 0..100 {
            storage_engine.add_data(memory_data.clone());
        }

        assert_eq!(storage_engine.get_channel_elements::<CpuStats>().unwrap().len(), 100);
        assert_eq!(storage_engine.get_channel_elements::<MemoryStats>().unwrap().len(), 100);
    }

    #[test]
    fn save_to_file_creates_file_with_data_test() {
        let dir = tempfile::tempdir().unwrap();

        let mut storage_engine = StorageEngine {
            storage_channels: std::collections::HashMap::default(),
            save_to_file_interval: std::cell::OnceCell::from(30),
            base_folder: std::cell::OnceCell::from(dir.path().to_path_buf()),
        };

        let cpu_data = CpuStats::get_current_stats();
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data.clone());
        storage_engine.add_data(cpu_data);

        storage_engine.save_to_file::<CpuStats>().unwrap();

        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap())
            .collect();

        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().to_str().unwrap().ends_with(".obs"));
        assert!(files[0].metadata().unwrap().len() > 0);

        // channel is drained after save
        assert!(storage_engine.get_channel_elements::<CpuStats>().is_none());
    }

    #[test]
    fn save_to_file_creates_file_with_big_junk_of_data_test() {
        let dir = tempfile::tempdir().unwrap();

        let mut storage_engine = StorageEngine {
            storage_channels: std::collections::HashMap::default(),
            save_to_file_interval: std::cell::OnceCell::from(30),
            base_folder: std::cell::OnceCell::from(dir.path().to_path_buf()),
        };

        let cpu_data = CpuStats::get_current_stats();
        for i in 0..10000 {
            storage_engine.add_data(cpu_data.clone());
        }

        storage_engine.save_to_file::<CpuStats>().unwrap();

        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap())
            .collect();

        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().to_str().unwrap().ends_with(".obs"));
        assert!(files[0].metadata().unwrap().len() > 0);

        // channel is drained after save
        assert!(storage_engine.get_channel_elements::<CpuStats>().is_none());
    }
}
