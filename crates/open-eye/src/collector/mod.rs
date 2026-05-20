pub mod container_runtime;
pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod processes;
pub mod speedtest;
pub mod systemstats;

pub trait DataCreationTime{
    fn get_data_creation_time(&self) -> i64;
}