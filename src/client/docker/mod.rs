pub mod collector;
pub mod docker_job;
pub mod docker_metric_sender;

struct DockerHealth {
    pub socket_available: bool,
    pub sender_working: bool,
}
