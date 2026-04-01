// makes this a library crate so benches (and tests) can import modules by name
// without this, only main.rs can access the code and `use observer::...` won't work

pub mod client;
pub mod config;
pub mod logging;
pub mod system_health;
pub mod scheduling;
