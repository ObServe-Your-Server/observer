use crate::notification::push_notification::PushNotification;
use anyhow::{Result, anyhow};
use reqwest::Client;
use reqwest::StatusCode;

#[derive(Clone)]
pub struct NotificationHandler {
    push_notification_url: String,
    api_key: String,
    machine_name: String,
}

impl NotificationHandler {
    pub fn new(
        push_notification_url: &str,
        api_key: &str,
        machine_name: &str,
    ) -> NotificationHandler {
        NotificationHandler {
            push_notification_url: push_notification_url.to_string(),
            api_key: api_key.to_string(),
            machine_name: machine_name.to_string(),
        }
    }
    pub async fn send_push_notification(&self, push_notification: &PushNotification) -> Result<()> {
        let client = Client::new();

        let push_notification = PushNotification {
            title: format!("{}: {}", self.machine_name, push_notification.title),
            body: push_notification.body.clone(),
        };

        let response = client
            .post(&self.push_notification_url)
            .header("X-Api-Key", &self.api_key)
            .json(&push_notification)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                log::info!("Sent notification: {}", push_notification);
                Ok(())
            }
            err => Err(anyhow!("Error sending notification: {}", err)),
        }
    }
}
