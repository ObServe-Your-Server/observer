use crate::notification::notification::Notification;
use crate::notification::notification_kind::NotificationKind;
use crate::notification::notification_urgency::NotificationUrgency;

pub struct MetricNotification {
    pub kind: NotificationKind,
    pub urgency: NotificationUrgency,
    pub title: String,
    pub body: String,
}

impl Notification for MetricNotification {
    fn kind(&self) -> &NotificationKind {
        &self.kind
    }

    fn urgency(&self) -> &NotificationUrgency {
        &self.urgency
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn body(&self) -> &str {
        &self.body
    }
}