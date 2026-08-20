use chrono::{DateTime, Utc};
use serde_json::Value;

pub struct TunnelAccessLogEntry {
    sent_data: Value,
    sent_at: DateTime<Utc>,
}
