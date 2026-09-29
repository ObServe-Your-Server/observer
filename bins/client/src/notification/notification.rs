use crate::notification::notification_kind::NotificationKind;
use crate::notification::notification_urgency::NotificationUrgency;

pub trait Notification {
    fn kind(&self) -> NotificationKind;
    fn urgency(&self) -> NotificationUrgency;
    fn title(&self) -> &str;
    fn body(&self) -> &str;
}