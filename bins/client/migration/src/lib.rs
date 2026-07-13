pub use sea_orm_migration::prelude::*;

mod m20260712_221402_cpu_stats;
mod m20260713_143522_container_runtime_stats;
mod m20260713_143538_disk_stats;
mod m20260713_143549_memory_stats;
mod m20260713_143558_network_stats;
mod m20260713_143612_process_stats;
mod m20260713_143619_speedtest_stats;
mod m20260713_143634_system_stats;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260712_221402_cpu_stats::Migration),
            Box::new(m20260713_143522_container_runtime_stats::Migration),
            Box::new(m20260713_143538_disk_stats::Migration),
            Box::new(m20260713_143549_memory_stats::Migration),
            Box::new(m20260713_143558_network_stats::Migration),
            Box::new(m20260713_143612_process_stats::Migration),
            Box::new(m20260713_143619_speedtest_stats::Migration),
            Box::new(m20260713_143634_system_stats::Migration),
        ]
    }
}
