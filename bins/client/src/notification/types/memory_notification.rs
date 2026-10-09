use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::notification::Notification;
use crate::notification::reporting::notification_delivery_mode::NotificationDeliveryMode;
use crate::notification::reporting::notification_usage_type::NotificationUsageType;
use crate::notification::reporting::reporter::Reporter;

#[derive(Clone)]
pub struct MemoryNotification {
    pub memory_usage_in_percent: u8,
    pub title: String,
    pub body: String,
}

impl Notification for MemoryNotification {
    fn reporter(&self) -> &Reporter {
        &Reporter::Memory
    }

    fn should_deliver_based_on_ruleset(
        &self,
        notification_config: &NotificationConfig,
    ) -> NotificationDeliveryMode {
        if !notification_config.enable_memory_notification {
            return NotificationDeliveryMode::DeactivatedFromConfig;
        }
        NotificationDeliveryMode::Decide
    }

    fn notification_usage_type(
        &self,
        notification_config: &NotificationConfig,
    ) -> NotificationUsageType {
        match notification_config.memory_high_percentage {
            None => {
                if self.memory_usage_in_percent >= 85 {
                    return NotificationUsageType::HighUsage;
                }
            }
            Some(target_pct) => {
                if self.memory_usage_in_percent >= target_pct {
                    return NotificationUsageType::HighUsage;
                }
            }
        }
        NotificationUsageType::NormaleState
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn body(&self) -> &str {
        &self.body
    }

    fn boxed_self(&self) -> Box<dyn Notification> {
        Box::new(self.clone())
    }
}
