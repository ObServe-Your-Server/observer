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

struct SentState {
    urgency: NotificationUrgency,
    suppressed_count: u16,
}

pub struct NotificationManager{
    notification_config: NotificationConfig,
    client_config: ClientConfig,
    last_sent: Mutex<HashMap<NotificationKind, SentState>>,
}

impl NotificationManager {
    pub fn notification_manager(notification_config: NotificationConfig,
                                         client_config: ClientConfig) -> NotificationManager{
        NotificationManager{
            notification_config,
            client_config,
            last_sent: Mutex::new(HashMap::new()),
        }
    }

    pub async fn send_notification(&self, notification: &dyn Notification) -> Result<()> {
        if !self.should_notification_be_sent(notification).await? {
            return Ok(())
        }

        match self.notification_config.notification_way {
            NotificationWay::PushNotification => self.send_push_notification(notification.title(), notification.body()).await?
        }

        let mut last_sent = tokio::time::timeout(LOCK_TIMEOUT, self.last_sent.lock())
            .await
            .map_err(|_| anyhow!("timed out waiting for notification state lock"))?;
        last_sent.insert(notification.kind(), SentState {
            urgency: notification.urgency(),
            suppressed_count: 0,
        });

        Ok(())
    }

    pub async fn send_push_notification(&self, title: &str, body: &str) -> Result<()> {
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
        let urgency = notification.urgency();
        if urgency == NotificationUrgency::AlwaysDeliver {
            return Ok(true);
        }

        let kind = notification.kind();
        let mode = self.notification_mode_for(kind);
        let renotify_after = self.renotify_after_for(kind);

        let mut last_sent = tokio::time::timeout(LOCK_TIMEOUT, self.last_sent.lock())
            .await
            .map_err(|_| anyhow!("timed out waiting for notification state lock"))?;

        Ok(match last_sent.get_mut(&kind) {
            None => true,
            Some(state) if state.urgency != urgency => true,
            Some(state) => match mode {
                NotificationMode::Continuous => true,
                NotificationMode::Once => match renotify_after {
                    None => false,
                    Some(threshold) => {
                        state.suppressed_count += 1;
                        state.suppressed_count >= threshold
                    }
                },
            },
        })
    }

    fn notification_mode_for(&self, kind: NotificationKind) -> NotificationMode {
        let mode = match kind {
            NotificationKind::Cpu => &self.notification_config.cpu_notification_mode,
            NotificationKind::Memory => &self.notification_config.memory_notification_mode,
            NotificationKind::Disk => &self.notification_config.disk_notification_mode,
            NotificationKind::ContainerSocket => &self.notification_config.container_socket_notification_mode,
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
