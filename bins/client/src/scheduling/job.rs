use anyhow::Result;
pub trait Job {
    async fn run(&self) -> Result<()>;
    fn name(&self) -> &str;
}