pub(crate) mod metrics;
mod metrics_mapping;
pub mod metrics_tunnel;
pub(crate) mod response_builder;

include!(concat!(env!("OUT_DIR"), "/observer.v1.rs"));