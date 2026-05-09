// needed structure because of the way the package is used in the prost auto gen part for tonic
pub mod observer {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/observer.v1.rs"));
        pub mod metrics {
            include!(concat!(env!("OUT_DIR"), "/observer.v1.metrics.rs"));
        }
    }
}

mod client;
