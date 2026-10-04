use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::reporting::notification_urgency::NotificationUrgency;
use crate::notification::reporting::reporter::Reporter;
use anyhow::Result;

pub trait Notification: Send + Sync {

    fn reporter(&self) -> &Reporter;
    fn apply_ruleset(&self, notification_config: &NotificationConfig) -> NotificationUrgency;
    fn title(&self) -> &str;
    fn body(&self) -> &str;
}