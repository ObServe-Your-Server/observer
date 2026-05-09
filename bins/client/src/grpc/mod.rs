pub mod observer {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/observer.v1.rs"));
        pub mod metrics {
            include!(concat!(env!("OUT_DIR"), "/observer.v1.metrics.rs"));
        }
    }
}

mod client;
