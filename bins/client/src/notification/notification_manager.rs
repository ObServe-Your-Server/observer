use std::cmp::PartialEq;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::notification_config::{NotificationConfig, NotificationMode, NotificationWay};
use crate::notification::notification::Notification;
use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use crate::notification::notification_kind::NotificationKind;
use crate::notification::notification_urgency::NotificationUrgency;

const LOCK_TIMEOUT: Duration = Duration::from_secs(1);

struct MetricState {
    confirmed: Option<NotificationUrgency>,
    candidate: NotificationUrgency,
    candidate_streak: u32,
    repeat_count: u16,
}

pub struct NotificationManager{
    notification_config: NotificationConfig,
    client_config: ClientConfig,
    metric_states: Mutex<HashMap<NotificationKind, MetricState>>,
}

impl NotificationManager {
    pub fn new(notification_config: NotificationConfig,
                                         client_config: ClientConfig) -> NotificationManager{
        NotificationManager{
            notification_config,
            client_config,
            metric_states: Mutex::new(HashMap::new()),
        }
    }

    pub async fn send_notification(&self, notification: &dyn Notification) -> Result<()> {
        if !self.should_notification_be_sent(notification).await? {
            return Ok(())
        }

        match self.notification_config.notification_way {
            NotificationWay::PushNotification => self.send_push_notification(notification.title(), notification.body()).await?
        }
        log::debug!("Sent notification: {} - {}", notification.title(), notification.body());

        Ok(())
    }

    async fn send_push_notification(&self, title: &str, body: &str) -> Result<()> {
        let client = Client::new();
        #[derive(Serialize)]
        pub struct PushNotification<'a> {
            pub title: &'a str,
            pub body: &'a str,
        }
        let push_notification = PushNotification {
            title,
            body
        };

        let response = client
            .post(&self.client_config.push_notification_url)
            .header("X-Api-Key", &self.client_config.api_key)
            .json(&push_notification)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                log::debug!("Sent push notification title: {} body: {}", push_notification.title, push_notification.body);
                Ok(())
            }
            err => {
                // TODO: log the error. If not registered on us, notify
                Err(anyhow!("Error sending notification: {}", err))
            },
        }
    }

    async fn should_notification_be_sent(&self, notification: &dyn Notification) -> Result<bool> {
        let kind = *notification.kind();
        let urgency = *notification.urgency();

        if !self.is_enabled(kind) {
            return Ok(false);
        }
        // Disks are debounced per mount point by the job that measures them.
        if urgency == NotificationUrgency::AlwaysDeliver || kind == NotificationKind::Disk {
            return Ok(true);
        }

        let mut states = tokio::time::timeout(LOCK_TIMEOUT, self.metric_states.lock())
            .await
            .map_err(|_| anyhow!("timed out waiting for notification state lock"))?;
        let state = states
            .entry(kind)
            .or_insert(MetricState {
                confirmed: None,
                candidate: urgency,
                candidate_streak: 0,
                repeat_count: 0,
            });

        if state.candidate == urgency {
            state.candidate_streak += 1;
        } else {
            state.candidate = urgency;
            state.candidate_streak = 1;
        }
        if state.candidate_streak < self.confirm_after(kind, urgency) {
            return Ok(false);
        }

        if state.confirmed != Some(urgency) {
            let first_status = state.confirmed.is_none();
            state.confirmed = Some(urgency);
            state.repeat_count = 0;
            return Ok(match urgency {
                NotificationUrgency::HighUsage => true,
                NotificationUrgency::LowUsage => !first_status,
                _ => false,
            });
        }

        // Status unchanged: only continuous mode repeats, and only while usage is high.
        if urgency != NotificationUrgency::HighUsage
            || self.notification_mode_for(kind) != NotificationMode::Continuous
        {
            return Ok(false);
        }
        let Some(renotify_after) = self.renotify_after_for(kind) else {
            return Ok(false);
        };

        state.repeat_count += 1;
        if state.repeat_count >= renotify_after {
            state.repeat_count = 0;
            return Ok(true);
        }
        Ok(false)
    }

    fn confirm_after(&self, kind: NotificationKind, urgency: NotificationUrgency) -> u32 {
        let config = &self.notification_config;
        let after = match (kind, urgency) {
            (NotificationKind::Cpu, NotificationUrgency::HighUsage) => config.cpu_high_after,
            (NotificationKind::Cpu, NotificationUrgency::LowUsage) => config.cpu_low_after,
            (NotificationKind::Memory, NotificationUrgency::HighUsage) => config.memory_high_after,
            (NotificationKind::Memory, NotificationUrgency::LowUsage) => config.memory_low_after,
            _ => None,
        };
        after.unwrap_or(1).max(1)
    }

    fn is_enabled(&self, kind: NotificationKind) -> bool {
        let config = &self.notification_config;
        match kind {
            NotificationKind::Cpu => config.enable_cpu_notification,
            NotificationKind::Memory => config.enable_memory_notification,
            NotificationKind::Disk => config.enable_disk_notification,
            NotificationKind::ContainerSocket => config.enable_advanced_container_socket_notifications,
        }
    }

    fn notification_mode_for(&self, kind: NotificationKind) -> NotificationMode {
        let mode = match kind {
            NotificationKind::Cpu => self.notification_config.cpu_notification_mode,
            NotificationKind::Memory => self.notification_config.memory_notification_mode,
            NotificationKind::Disk => Some(NotificationMode::Once),
            NotificationKind::ContainerSocket => self.notification_config.container_socket_notification_mode,
        };
        mode.unwrap_or(NotificationMode::Once)
    }

    fn renotify_after_for(&self, kind: NotificationKind) -> Option<u16> {
        match kind {
            NotificationKind::Cpu => self.notification_config.cpu_renotify_after_x,
            NotificationKind::Memory => self.notification_config.memory_renotify_after_x,
            NotificationKind::Disk => self.notification_config.disk_renotify_after_x,
            NotificationKind::ContainerSocket => self.notification_config.container_socket_renotify_after_x,
        }
    }
}
