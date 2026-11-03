use crate::grpc::v1::TimeQuery;
use crate::grpc::v1::time_query::Query;
use chrono::{DateTime, TimeZone, Utc};

#[derive(Clone, Copy, Debug)]
pub enum QueryRange {
    Between(DateTime<Utc>, DateTime<Utc>),
    LastN(u64),
}

impl QueryRange {
    /// Builds the range from the `time_query` of a single request.
    /// A missing query falls back to an empty range (`now..now`).
    pub fn from_time_query(time_query: Option<&TimeQuery>) -> Self {
        match time_query.and_then(|q| q.query.as_ref()) {
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
