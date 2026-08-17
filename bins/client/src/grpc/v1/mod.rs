mod metrics_mapping;
pub mod metrics_tunnel;
pub(crate) mod response_builder;
mod metrics;

include!(concat!(env!("OUT_DIR"), "/observer.v1.rs"));