use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::reporting::notification_delivery_mode::NotificationDeliveryMode;
use crate::notification::reporting::notification_usage_type::NotificationUsageType;
use crate::notification::reporting::reporter::Reporter;

pub trait Notification: Send + Sync {
    fn reporter(&self) -> &Reporter;
    fn should_deliver_based_on_ruleset(
        &self,
        notification_config: &NotificationConfig,
    ) -> NotificationDeliveryMode;
    fn notification_usage_type(
        &self,
        notification_config: &NotificationConfig,
    ) -> NotificationUsageType;
    fn title(&self) -> &str;
    fn body(&self) -> &str;
    fn boxed_self(&self) -> Box<dyn Notification>;
}
