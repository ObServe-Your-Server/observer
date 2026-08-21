use serde::Serialize;

#[derive(Serialize)]
pub struct PushNotification {
    pub title: String,
    pub body: String,
}