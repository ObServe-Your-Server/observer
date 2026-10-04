#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUrgency{
    DontDeliver,
    LowUsage,
    HighUsage,
    AlwaysDeliver
}