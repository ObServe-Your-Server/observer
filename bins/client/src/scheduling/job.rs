use anyhow::Result;
use chrono::Duration;

#[async_trait::async_trait]
pub trait Job: Send + Sync {
    async fn run(&self) -> Result<()>;
    fn schedule_time(&self) -> Duration;
    fn name(&self) -> &str;
}