
pub mod collector;
pub mod docker_metric_sender;
pub mod docker_job;

struct DockerHealth{
    pub socket_available: bool,
    pub sender_working: bool,
}