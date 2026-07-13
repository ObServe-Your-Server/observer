use anyhow::Result;
use chrono::Duration;

#[async_trait::async_trait]
pub trait Job {
    async fn run(&self) -> Result<()>;
    fn schedule_time(&self) -> Duration;
    fn name(&self) -> &str;
    fn increase_err_count(&mut self);
    fn max_err_count(&self) -> u16;
    fn current_err_count(&self) -> u16;
}