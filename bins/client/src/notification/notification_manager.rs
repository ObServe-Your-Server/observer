use crate::config::config_parts::client_config::ClientConfig;
use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::push_notification::push_notification::PushNotification;
use crate::notification::push_notification::push_notification_handler::PushNotificationHandler;

pub struct NotificationManager{
    notification_config: NotificationConfig,
    client_config: ClientConfig,
    push_notification_handler: PushNotificationHandler,
}

impl NotificationManager {
    pub fn new_push_notification_manager(notification_config: NotificationConfig,
                                         client_config: ClientConfig,
                                         push_notification_handler: PushNotificationHandler) -> NotificationManager {
        NotificationManager{
            notification_config,
            client_config,
            push_notification_handler,
        }

    }

    pub fn send_push_notification(push_notification: PushNotification){

    }
}