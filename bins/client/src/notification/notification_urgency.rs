#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUrgency{
    LowUsage,
    MediumUsage,
    HighUsage,
    AlwaysDeliver
}