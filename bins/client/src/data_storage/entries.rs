use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::memory::collector::MemoryStats;
use crate::data_storage::storage_engine::DataBlockEntry;

impl DataBlockEntry for CpuStats {}

impl DataBlockEntry for MemoryStats {}