use crate::notification::push_notification::push_notification::PushNotification;

pub enum Notification {
    Push(PushNotification)
}