use crate::scheduling::job::{Job, JobTrait};
use anyhow::{Result, anyhow};
use std::time::{Duration, Instant};
use log::error;
use tokio::task::JoinSet;
use tokio::time;
use tokio::time::MissedTickBehavior;

pub struct Scheduler {
    job_list: Vec<Job>,
}

impl Scheduler {
    pub fn new(job_list: Vec<Job>) -> Scheduler {
        Scheduler { job_list }
    }

    pub fn add_job(&mut self, job: Job) {
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

async fn run(mut job: Job) -> Result<()> {
    log::info!(
        "Scheduler [{}] starting, running every {}s",
        job.name(),
        job.interval().as_seconds_f64()
    );

    let duration = Duration::from_secs_f64(job.interval().as_seconds_f64());
    let mut interval = time::interval(duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut timer: Instant;
    loop {
        interval.tick().await;

        // reset the timer
        timer = Instant::now();

        // run the job and wait on the result
        match time::timeout(duration, job.task().run()).await {
            Ok(Ok(_)) => {
                log::debug!(
                    "Job: {} run successfully. Duration: {:.3}s",
                    job.name(),
                    timer.elapsed().as_secs_f32()
                );
            }
            Ok(Err(e)) => Err(anyhow!("Job failed with error: {}", e))?,
            Err(_) => {
                // timeout
                Err(anyhow!("Job timeouted"))?
            }
        }
    }
}
