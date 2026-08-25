use chrono::{DateTime, TimeZone, Utc};
use crate::grpc::v1::metrics_request::Query;
use crate::grpc::v1::MetricsRequest;


#[derive(Clone, Copy, Debug)]
pub enum QueryRange {
    Between(DateTime<Utc>, DateTime<Utc>),
    LastN(u64),
}

impl QueryRange {
    pub fn from_request(request: &MetricsRequest) -> Self {
        match &request.query {
            Some(Query::LastN(n)) => QueryRange::LastN((*n).max(0) as u64),
            Some(Query::Range(range)) => {
                let start = Utc
                    .timestamp_opt(range.start, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                let end = Utc
                    .timestamp_opt(range.end, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                QueryRange::Between(start, end)
            }
            None => QueryRange::Between(Utc::now(), Utc::now()),
        }
    }
}