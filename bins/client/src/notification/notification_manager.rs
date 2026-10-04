use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::notification_config::{NotificationConfig, NotificationWay};
use crate::notification::notification::Notification;
use crate::notification::reporting::notification_urgency::NotificationUrgency;
use crate::notification::reporting::reporter::Reporter;
use anyhow::{Result, anyhow};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use log::__private_api::loc;
use tokio::sync::Mutex;
use tokio::time::timeout;

const LOCK_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone)]
struct MetricState {
    urgency: NotificationUrgency,
    repeated_report: u64,
}

pub struct NotificationManager{
    notification_config: NotificationConfig,
    client_config: ClientConfig,
    metric_states: Mutex<BTreeMap<Reporter, MetricState>>,
}

impl NotificationManager {
    pub fn new(notification_config: NotificationConfig,
                                         client_config: ClientConfig) -> NotificationManager{
        NotificationManager{
            notification_config,
            client_config,
            metric_states: Mutex::new(BTreeMap::new()),
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
        // first apply ruleset embedded in notification
        let urgency = notification.apply_ruleset(&self.notification_config);
        if urgency == NotificationUrgency::DontDeliver {
            self.remove_entry(notification).await?;
            return Ok(false)
        }
        if urgency == NotificationUrgency::AlwaysDeliver {
            return Ok(true)
        }

        // evaluate rest base of notify after x etc


        todo!()
    }

    fn get_target_repetition_count(&self, reporter: &Reporter, urgency: NotificationUrgency) -> Option<u32> {
        match reporter {
            Reporter::System => None,
            Reporter::Cpu => {
                // TODO
            }
            Reporter::Memory => {}
            Reporter::Disk => {}
            Reporter::ContainerSocket => {}
        }
        todo!()
    }

    async fn get_entry(&self, reporter: &Reporter) -> Result<Option<MetricState>> {
        match timeout(LOCK_TIMEOUT, self.metric_states.lock()).await {
            Ok(states) => {
                match states.get(reporter) {
                    None => Ok(None),
                    Some(removed) => Ok(Some(removed.clone()))
                }
            }
            Err(_) => Err(anyhow!("Unable to acquire lock for metric_states")),
        }
    }

    async fn remove_entry(&self, notification: &dyn Notification) -> Result<Option<MetricState>> {
        match timeout(LOCK_TIMEOUT, self.metric_states.lock()).await {
            Ok(mut states) => {
            match states.remove(notification.reporter()) {
                None => Ok(None),
                Some(removed) => Ok(Some(removed))
            }
            }
            Err(_) => Err(anyhow!("Unable to acquire lock for metric_states")),
        }
    }
}
