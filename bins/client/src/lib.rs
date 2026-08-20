// makes this a library crate so benches (and tests) can import modules by name
// without this, only main.rs can access the code and `use observer::...` won't work

mod access_logging;
pub mod config;
pub mod entities;
mod error_handling;
mod grpc;
pub mod jobs;
pub mod logging;
mod notification;
pub mod scheduling;
mod storage_engine;

#[cfg(test)]
mod test_support;
