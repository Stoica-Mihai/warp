//! Supporting types for persisting cloud objects to SQLite.

#[cfg(test)]
pub use warp_server_client::persistence::{decode_guests, encode_guests};

#[cfg(test)]
#[path = "cloud_object_tests.rs"]
mod tests;
