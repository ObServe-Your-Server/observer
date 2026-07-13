use chrono::{DateTime, Utc};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tokio::time::MissedTickBehavior;
use crate::scheduling::job::Job;

pub struct Scheduler {
    interval_secs: u32,
}

impl Scheduler {

    pub async fn run<J: Job>(mut self, job: J)
    {
        todo!();
        let duration = Duration::from_secs(self.interval_secs as u64);
        let mut interval = time::interval(duration);
        // skip if the execution took too long or other issues
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        log::info!(
            "Scheduler [{}] starting, running every {}s",
            job.name(),
            self.interval_secs
        );

        loop {
            interval.tick().await;

            // run the job and wait on the result
            match time::timeout(duration, job.run()).await {
                Ok(Ok(_)) => {

                }
                Ok(Err(e)) => {
                    log::error!("Scheduler [{}] job failed: {}", job.name(), e);
                    // handle the error from the job
                }
                Err(_) => {
                    // executes when the job takes too long
                    todo!()
                }
            }
        }
    }
}
