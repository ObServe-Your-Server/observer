#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationDeliveryMode {
    DeactivatedFromConfig,
    AlwaysDeliver,
    Decide
}