pub(crate) mod metrics;
mod metrics_mapping;
pub mod metrics_tunnel;

include!(concat!(env!("OUT_DIR"), "/observer.v1.rs"));