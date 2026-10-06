use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::notification_config::{NotificationConfig, NotificationWay};
use crate::notification::notification::Notification;
use crate::notification::reporting::notification_delivery_type::NotificationDeliveryType;
use crate::notification::reporting::reporter::Reporter;
use anyhow::{Result, anyhow};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use crate::notification::reporting::notification_usage_type::NotificationUsageType;

const LOCK_TIMEOUT: Duration = Duration::from_millis(250);

pub struct NotificationManager{
    notification_config: NotificationConfig,
    client_config: ClientConfig,
    metric_states: Mutex<BTreeMap<Reporter, Vec<Box<dyn Notification>>>>,
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
        if !self.should_notification_be_send(notification).await? {
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

    async fn should_notification_be_send(&self, notification: &dyn Notification) -> Result<bool> {
        // first apply ruleset embedded in notification
        let urgency = notification.should_deliver_based_on_ruleset(&self.notification_config);

        match urgency {
            NotificationDeliveryType::DeactivatedFromConfig => Ok(false),
            NotificationDeliveryType::AlwaysDeliver => Ok(true),
            NotificationDeliveryType::Decide => {
                let target_repetition_count = self.get_target_repetition_count(notification.reporter());
                // when no count set instant deliver
                if target_repetition_count == 0 {
                    return Ok(true)
                }

                // work with queue
                self.insert_notification_into_states_and_delete_old_entry(notification.boxed_self()).await?;
                self.should_notification_send_based_on_states_vec(notification.reporter()).await
            }
        }
    }

    async fn should_notification_send_based_on_states_vec(&self, reporter: &Reporter) -> Result<bool> {
        let notification_vec = match self.get_notification_list_for_type(reporter).await? {
            None => return Ok(false),
            Some(vec) => vec,
        };

        let target_repetition = self.get_target_repetition_count(reporter) as usize;

        // smaller than target so no change can be detected
        if notification_vec.len() < target_repetition {
            return Ok(false)
        }

        // now bigger than target now check first that subframe is uniform
        let frame = &notification_vec[notification_vec.len() - target_repetition..];
        let first_entry_in_subframe = match frame.first() {
            None => return Err(anyhow!("Code Err: No entry as fist of subvec for notification.")),
            Some(first) => first
        };
        for entry in frame {
            if entry.notification_usage_type(&self.notification_config) != first_entry_in_subframe.notification_usage_type(&self.notification_config) {
                // the timeframe is not uniform so no send
                return Ok(false)
            }
        }

        // now timeframe is uniform then check if it matches the first in the whole vec
        // if not notification should be sent
        match notification_vec.first() {
            None => return Err(anyhow!("Code Err: No first entry to check on.")),
            Some(overall_first) => {
                if overall_first.notification_usage_type(&self.notification_config) != first_entry_in_subframe.notification_usage_type(&self.notification_config){
                    return Ok(true)
                }
            }
        }
        Ok(false)
    }

    async fn get_notification_list_for_type(&self, reporter: &Reporter) -> Result<Option<Vec<Box<dyn Notification>>>> {
        match timeout(LOCK_TIMEOUT, self.metric_states.lock()).await {
            Ok(states) => {
                Ok(states
                    .get(reporter)
                    .map(|v| v.iter().map(|n| n.boxed_self()).collect()))
            }
            Err(_) => Err(anyhow!("Unable to acquire lock for metric_states")),
        }
    }


    /// Give the reporting system and an urgency.
    /// Returns the number for the queueing system
    fn get_target_repetition_count(&self, reporter: &Reporter) -> u32 {
        match reporter {
            Reporter::System => 0,
            Reporter::Cpu => self.notification_config.cpu_notify_after.unwrap_or(0) as u32,
            Reporter::Memory => self.notification_config.memory_notify_after.unwrap_or(0) as u32,
            Reporter::Disk => self.notification_config.disk_notify_after.unwrap_or(0) as u32,
            Reporter::ContainerSocket => todo!()
        }
    }


    async fn insert_notification_into_states_and_delete_old_entry(&self, notification: Box<dyn Notification>) -> Result<()> {
        let reporter = notification.reporter().clone();
        // window = the frame that has to be uniform + one entry before it as the comparison baseline
        let max_len = self.get_target_repetition_count(&reporter) as usize + 1;

        match timeout(LOCK_TIMEOUT, self.metric_states.lock()).await {
            Ok(mut states) => {
                let entries = states.entry(reporter).or_default();
                entries.push(notification);
                Self::remove_oldest_over_limit(entries, max_len);
                Ok(())
            }
            Err(_) => Err(anyhow!("Unable to acquire lock for metric_states")),
        }
    }

    /// Removes the oldest entries from the front until at most `max_len` are left.
    fn remove_oldest_over_limit(entries: &mut Vec<Box<dyn Notification>>, max_len: usize) {
        if entries.len() > max_len {
            entries.drain(..entries.len() - max_len);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::notification::types::cpu_notification::CpuNotification;
    use crate::notification::types::memory_notification::MemoryNotification;
    use super::*;

    fn manager() -> NotificationManager {
        let notification_config = NotificationConfig {
            notification_way: NotificationWay::PushNotification,
            enable_cpu_notification: true,
            cpu_notify_after: Some(3),
            cpu_high_percentage: Some(85),
            enable_memory_notification: false,
            memory_notify_after: None,
            memory_high_percentage: None,
            enable_disk_notification: false,
            disk_notify_after: None,
            disk_high_percentage: None,
            enable_container_socket_notifications: false,
            notify_on_high_container_socket_usage: None,
            container_socket_high_cpu_usage_percent: None,
            container_socket_low_cpu_usage_percent: None,
            container_socket_notify_on_container_down: None,
        };
        let client_config = ClientConfig {
            machine_name: None,
            base_server_grpc_url: String::new(),
            base_server_http_url: String::new(),
            push_notification_url: String::new(),
            api_key: String::new(),
            enable_container_metrics_collector: false,
        };
        NotificationManager::new(notification_config, client_config)
    }

    async fn insert_cpu(manager: &NotificationManager, cpu_usage_in_percent: u8) {
        let notification = CpuNotification {
            cpu_usage_in_percent,
            title: "cpu".to_string(),
            body: "cpu".to_string(),
        };
        manager
            .insert_notification_into_states_and_delete_old_entry(Box::new(notification))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sends_when_last_n_are_uniform_and_differ_from_baseline() {
        let manager = manager();
        // baseline normal, then 3 (= cpu_notify_after) high in a row
        for usage in [30, 90, 90, 90] {
            insert_cpu(&manager, usage).await;
        }

        let should_send = manager
            .should_notification_send_based_on_states_vec(&Reporter::Cpu)
            .await
            .unwrap();
        assert!(should_send);
    }

    #[tokio::test]
    async fn does_not_send_when_last_n_are_not_uniform() {
        let manager = manager();
        // high is interrupted by a normal value, so the last 3 are mixed
        for usage in [10, 90, 10, 90] {
            insert_cpu(&manager, usage).await;
        }

        let should_send = manager
            .should_notification_send_based_on_states_vec(&Reporter::Cpu)
            .await
            .unwrap();
        assert!(!should_send);
    }

    #[tokio::test]
    async fn does_not_send_when_memory_notifications_are_deactivated_in_config() {
        // manager() has enable_memory_notification: false
        let manager = manager();
        let notification = MemoryNotification {
            memory_usage_in_percent: 99,
            title: "memory".to_string(),
            body: "memory".to_string(),
        };

        let should_send = manager.should_notification_be_send(&notification).await.unwrap();
        assert!(!should_send);

        // deactivated notifications must not be queued either
        let queued = manager.get_notification_list_for_type(&Reporter::Memory).await.unwrap();
        assert!(queued.is_none());
    }
}
