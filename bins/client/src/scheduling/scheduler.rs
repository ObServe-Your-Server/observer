use crate::scheduling::job::Job;
use chrono::{DateTime, Utc};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tokio::time::MissedTickBehavior;
use anyhow::{anyhow, Result};
use tokio::task::JoinSet;

pub struct Scheduler {
    job_list: Vec<SchedulableJob>
}

pub struct SchedulableJob{
    job: Box<dyn Job>,
    error_count: u32,
    max_error_count: u32,
}

impl SchedulableJob {
    pub fn new(job: Box<dyn Job>, max_error_count: u32) -> SchedulableJob {
        SchedulableJob {
            job,
            error_count: 0,
            max_error_count,
        }
    }
}

impl Scheduler {
    pub fn new(job_list: Vec<SchedulableJob>) -> Scheduler {
        Scheduler{job_list}
    }

    pub fn add_job(&mut self, job: SchedulableJob){
        self.job_list.push(job);
    }

    pub async fn start_jobs_blocking(&mut self) -> Result<()> {
        if self.job_list.is_empty() {
            return Err(anyhow!("Job list is empty"));
        }

        let mut set = JoinSet::new();
        for job in self.job_list.drain(..) {
            set.spawn(run(job));
        }

        // if one job fails or returns then the match triggers
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => continue,
                Ok(Err(e)) => {
                    set.abort_all();
                    return Err(e);
                }
                Err(join_err) => {
                    set.abort_all();
                    return Err(anyhow!("job task panicked: {join_err}"));
                }
            }
        }

        Ok(())
    }
}

async fn run(mut schedulable_job: SchedulableJob) -> Result<()>{
    let job = &schedulable_job.job;
    log::info!(
            "Scheduler [{}] starting, running every {}s",
            job.name(),
            job.schedule_time().as_seconds_f64()
        );

    let duration = Duration::from_secs_f64(job.schedule_time().as_seconds_f64());
    let mut interval = time::interval(duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        // run the job and wait on the result
        match time::timeout(duration, job.run()).await {
            Ok(Ok(_)) => {
                log::info!("Job: {} run successfully.", job.name());
                schedulable_job.error_count = 0;
            }
            Ok(Err(e)) => {
                log::error!("Scheduler [{}] job failed: {}", job.name(), e);
                schedulable_job.error_count += 1;
                if schedulable_job.error_count >= schedulable_job.max_error_count {
                    return Err(anyhow!(
                        "job [{}] reached max error count ({}), terminating: {e}",
                        job.name(),
                        schedulable_job.max_error_count
                    ));
                }
            }
            Err(_) => {
                // executes when the job takes too long
                log::error!("Scheduler [{}] job timed out after {:?}", job.name(), duration);
                schedulable_job.error_count += 1;
                if schedulable_job.error_count >= schedulable_job.max_error_count {
                    return Err(anyhow!(
                        "job [{}] reached max error count ({}), terminating: timed out after {:?}",
                        job.name(),
                        schedulable_job.max_error_count,
                        duration
                    ));
                }
            }
        }
    }
}
