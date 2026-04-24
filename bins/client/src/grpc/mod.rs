pub mod metrics_proto {
    include!(concat!(env!("OUT_DIR"), "/metrics.rs"));
}

mod proto_handler;
pub use proto_handler::to_full_metrics;
