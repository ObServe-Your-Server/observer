pub mod metrics_proto {
    include!(concat!(env!("OUT_DIR"), "/metrics.rs"));
}

mod proto_mapper;
mod sender;

pub use proto_mapper::to_full_metrics;
