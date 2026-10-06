#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationDeliveryType {
    DeactivatedFromConfig,
    AlwaysDeliver,
    Decide
}