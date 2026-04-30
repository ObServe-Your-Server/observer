use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
struct Cpu {
    pub cpu_name: String,
    pub cpu_count: u16,
    pub cpu_physical_count: u16,
    pub cpu_usage_percent: f32,
    pub cpu_temperature_celsius: f32,
    pub core_information: Vec<Core>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Core {
    pub core_name: String,
    pub core_usage_percent: f32,
    pub core_frequency_mhz: u64,
}