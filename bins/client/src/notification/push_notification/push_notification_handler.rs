use anyhow::{Result, anyhow};
use reqwest::{Client, StatusCode};
use crate::notification::push_notification::push_notification::PushNotification;

pub struct PushNotificationHandler {
    push_notification_url: String,
    api_key: String,
}

impl PushNotificationHandler {
    pub fn new(push_notification_url: impl Into<String>, api_key: impl Into<String>) -> PushNotificationHandler {
        PushNotificationHandler{
            push_notification_url: push_notification_url.into(),
            api_key: api_key.into(),
        }
    }
    
    pub async fn send_notification(&self, push_notification: PushNotification) -> Result<()> {
        let client = Client::new();

        let response = client
            .post(&self.push_notification_url)
            .header("X-Api-Key", &self.api_key)
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