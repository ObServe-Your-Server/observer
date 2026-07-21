use std::fmt;
use std::fmt::{Debug, Formatter};
use reqwest::Client;
use serde::Serialize;
use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use sea_orm::ColIdx;
use tokio::fs::read_to_string;
use tonic::service::LayerExt;

#[derive(Clone)]
pub struct NotificationHandler {
    push_notification_url: String,
    api_key: String,
    machine_name: String,
}

#[derive(Serialize)]
pub struct PushNotification{
    pub title: String,
    pub body: String,
}

impl fmt::Display for PushNotification {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("Title: {}, body: {}", self.title, self.body))
    }
}

impl NotificationHandler {
    pub fn new(push_notification_url: String, api_key: String, machine_name: String) -> NotificationHandler {
        NotificationHandler{
            push_notification_url,
            api_key,
            machine_name
        }
    }
    pub async fn send_push_notification(&self, push_notification: &PushNotification) -> Result<()>{
        let client = Client::new();

        let push_notification = PushNotification{
            title: format!("{}: {}",self.machine_name, push_notification.body),
            body: push_notification.body.clone(),
        };

        let response = client.post(&self.push_notification_url)
            .header("X-Api-Key", &self.api_key)
            .json(&push_notification)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                log::info!("Sent notification: {}", push_notification);
                Ok(())
            },
            err => Err(anyhow!("Error sending notification: {}", err)),
        }
    }
}