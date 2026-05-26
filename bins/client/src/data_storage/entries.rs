use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::memory::collector::MemoryStats;
use crate::data_storage::storage_engine::DataBlockEntry;

// needed mapper for the structs because data block entry consists of erased-serde and the other traits

impl DataBlockEntry for CpuStats {}

impl DataBlockEntry for MemoryStats {}