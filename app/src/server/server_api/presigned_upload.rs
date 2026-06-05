use thiserror::Error;

/// Typed error for HTTP-backed operations so downstream classifiers can
/// decide transient vs permanent failures without string-parsing anyhow Display.
#[derive(Debug, Error)]
#[error("HTTP request failed with status {status}: {body}")]
pub struct HttpStatusError {
    pub status: u16,
    pub body: String,
}
