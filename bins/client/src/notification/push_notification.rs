use serde::Serialize;
use std::fmt;
use std::fmt::Formatter;

#[derive(Serialize)]
pub struct PushNotification {
    pub title: String,
    pub body: String,
}

impl fmt::Display for PushNotification {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("Title: {}, body: {}", self.title, self.body))
    }
}
