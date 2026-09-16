use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::notification::Notification;
use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use crate::notification::push_notification::PushNotification;

pub struct NotificationManager{
    notification_config: NotificationConfig,
    client_config: ClientConfig,
}

impl NotificationManager {
    pub fn new_push_notification_manager(notification_config: NotificationConfig,
                                         client_config: ClientConfig) -> NotificationManager{
        NotificationManager{
            notification_config,
            client_config,
        }
    }

    pub async fn send_notification(&self, notification: Notification) -> Result<()> {
        match notification {
            Notification::Push(push_notification) => self.send_push_notification(push_notification).await
        }
    }

    pub async fn send_push_notification(&self, push_notification: PushNotification) -> Result<()> {
        let client = Client::new();

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
            err => Err(anyhow!("Error sending notification: {}", err)),
        }
    }
}