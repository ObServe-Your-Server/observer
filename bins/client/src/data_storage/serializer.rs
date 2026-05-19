use std::fs::{self, File};
use std::io::Write;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct Serializer {}

const FILE_EXTENSION: &str = "obsr";


impl Serializer {

    pub fn serialize<D: Serialize>(data: &D) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        let mut buf = Vec::new();
        match data.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => Ok(buf),
            Err(err) => Err(err),
        }
    }

    pub fn deserialize<'a, D: Deserialize<'a>>(data: &'a [u8]) -> Result<D, rmp_serde::decode::Error> {
        rmp_serde::from_slice::<D>(data)
    }
}

#[cfg(test)]
mod tests {
    use open_eye::collector::cpu::collector::CpuStats;
    use crate::data_storage::serializer::Serializer;

    #[test]
    fn serialize_deserialize_cpu_metrics_test() {
        let cpu_metrics = CpuStats::get_current_stats();
        let cpu_metrics_serialized = Serializer::serialize(&cpu_metrics).unwrap();

        println!("Serialized length: {:?}", cpu_metrics_serialized.len());
        println!("Serialized metrics: {:?}", cpu_metrics_serialized);

        let cpu_metrics_deserialized = Serializer::deserialize::<CpuStats>(&cpu_metrics_serialized).unwrap();

        println!("Deserialized data: {:#?}", cpu_metrics_deserialized);

        assert_eq!(cpu_metrics, cpu_metrics_deserialized);
    }
}
