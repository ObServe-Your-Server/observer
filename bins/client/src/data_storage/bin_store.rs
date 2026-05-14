use std::fs::{self, File};
use std::io::Write;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::data_storage::BinStoreError;

pub struct BinStore {}

const FILE_EXTENSION: &str = "obsr";

macro_rules! check_observe_fileextension {
    ($path:expr) => {
        if $path.extension().and_then(|e| e.to_str()) != Some(FILE_EXTENSION) {
            return Err(BinStoreError::InvalidExtension);
        }
    };
}

impl BinStore {
    pub fn save_to_new_file<D: Serialize>(file_path: &PathBuf, data: D) -> Result<Vec<u8>, BinStoreError> {
        check_observe_fileextension!(file_path);
        let bytes = Self::serialize(data)?;
        let mut file = File::create(file_path)?;
        file.write_all(&bytes)?;
        Ok(bytes)
    }

    pub fn append_to_file<D: Serialize + DeserializeOwned>(file_path: &PathBuf, data: D) -> Result<Vec<u8>, BinStoreError> {
        check_observe_fileextension!(file_path);
        if !file_path.exists() {
            return Err(BinStoreError::FileNotFound);
        }
        let data_in_file = Self::load_from_file::<D>(file_path);
        todo!()
    }

    pub fn load_from_file<D: DeserializeOwned>(file_path: &PathBuf) -> Result<D, BinStoreError> {
        check_observe_fileextension!(file_path);
        let bytes = fs::read(file_path)?;
        Ok(rmp_serde::from_slice(&bytes)?)
    }

    fn serialize<D: Serialize>(data: D) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        let mut buf = Vec::new();
        match data.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => Ok(buf),
            Err(err) => Err(err),
        }
    }

    fn deserialize<'a, D: Deserialize<'a>>(data: &'a [u8]) -> Result<D, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use open_eye::collector::cpu::collector::CpuStats;
    use crate::data_storage::bin_store::BinStore;
    use crate::data_storage::BinStoreError;

    #[test]
    fn serialize_deserialize_cpu_metrics_test() {
        let cpu_metrics = CpuStats::get_current_stats();
        let cpu_metrics_serialized = BinStore::serialize(&cpu_metrics).unwrap();

        println!("Serialized length: {:?}", cpu_metrics_serialized.len());
        println!("Serialized metrics: {:?}", cpu_metrics_serialized);

        let cpu_metrics_deserialized = BinStore::deserialize::<CpuStats>(&cpu_metrics_serialized).unwrap();

        println!("Deserialized data: {:#?}", cpu_metrics_deserialized);

        assert_eq!(cpu_metrics, cpu_metrics_deserialized);
    }

    #[test]
    fn save_cpu_metrics_test(){
        let tmp_dir = TempDir::new("observe").unwrap();
        let file_path = tmp_dir.path().join("cpu_metrics.obsr");
        let cpu_metrics = CpuStats::get_current_stats();

        BinStore::save_to_new_file(&file_path, &cpu_metrics).unwrap();

        let cpu_metrics_from_file = BinStore::load_from_file::<CpuStats>(&file_path).unwrap();

        assert_eq!(cpu_metrics, cpu_metrics_from_file);

    }

    #[test]
    fn wrong_file_extension_test(){
        let tmp_dir = TempDir::new("observe").unwrap();
        let file_path = tmp_dir.path().join("cpu_metrics.wrong");
        let cpu_metrics = CpuStats::get_current_stats();

        let res = BinStore::save_to_new_file(&file_path, &cpu_metrics);

        assert!(matches!(res, Err(BinStoreError::InvalidExtension)))

    }



}
