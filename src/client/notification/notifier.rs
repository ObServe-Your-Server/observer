pub trait Notification {
    fn get_message(&self) -> String;
    fn get_subject(&self) -> String;
}

pub struct CpuNotification {
    pub cpu_usage: f32,
}