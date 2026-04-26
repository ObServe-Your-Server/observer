pub mod connection_proto {
    include!(concat!(env!("OUT_DIR"), "/connection.rs"));
}

mod sender;

pub use sender::Sender;
