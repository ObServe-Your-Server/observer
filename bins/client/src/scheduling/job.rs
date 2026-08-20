use anyhow::Result;
use chrono::Duration;
use getset::Getters;

#[derive(Getters)]
pub struct Job {
    #[getset(get = "pub")]
    name: String,
    task: Box<dyn JobTrait>,
    #[getset(get = "pub")]
    interval: Duration,
    #[getset(get = "pub")]
    fatal_errors_before_termination: u16,
}

impl Job {
    pub fn new(name: &str, task: Box<dyn JobTrait>, interval: Duration, fatal_errors_before_termination: u16) -> Job {
        Job{
            name: name.to_string(),
            task,
            interval,
            fatal_errors_before_termination
        }
    }

    pub fn task(&self) -> &dyn JobTrait {
        self.task.as_ref()
    }
}

#[async_trait::async_trait]
pub trait JobTrait: Send + Sync {
    async fn run(&self) -> Result<()>;
}