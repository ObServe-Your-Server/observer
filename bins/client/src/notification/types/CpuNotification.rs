use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::notification::Notification;
use crate::notification::reporting::notification_urgency::NotificationUrgency;
use crate::notification::reporting::reporter::Reporter;

pub struct CpuNotification{
    pub cpt_usage: u8,
    pub title: String,
    pub body: String,
}

impl Notification for CpuNotification {

    fn reporter(&self) -> &Reporter{
        &Reporter::Cpu
    }

    fn apply_ruleset(&self, notification_config: &NotificationConfig) -> NotificationUrgency {
        if !notification_config.enable_cpu_notification {
            return NotificationUrgency::DontDeliver;
        }

        // check high usage
        match notification_config.cpu_high_percentage {
            None => return NotificationUrgency::DontDeliver,
            Some(target_pct) => {
                if self.cpt_usage >= target_pct { return NotificationUrgency::HighUsage }
            }
        }

        // check low usage
        match notification_config.cpu_low_percentage {
            None => return NotificationUrgency::DontDeliver,
            Some(target_pct) => {
                if self.cpt_usage <= target_pct { return NotificationUrgency::LowUsage }
            }
        }

        NotificationUrgency::DontDeliver
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn body(&self) -> &str {
        &self.title
    }
}