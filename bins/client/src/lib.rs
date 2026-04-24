// makes this a library crate so benches (and tests) can import modules by name
// without this, only main.rs can access the code and `use observer::...` won't work

pub mod config;
pub mod logging;
mod mapper;
pub mod metrics_sender;
pub mod scheduling;
mod sender;
mod subsystem;
pub mod system_health;

mod grpc;