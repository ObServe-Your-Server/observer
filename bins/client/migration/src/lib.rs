pub use sea_orm_migration::prelude::*;

mod m20260712_221402_cpu_stas;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260712_221402_cpu_stas::Migration),
        ]
    }
}
