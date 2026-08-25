mod metrics;
mod metrics_mapping;
pub mod metrics_tunnel;
pub(crate) mod response_builder;
mod metrics_server;
mod query_range;

include!(concat!(env!("OUT_DIR"), "/observer.v1.rs"));
