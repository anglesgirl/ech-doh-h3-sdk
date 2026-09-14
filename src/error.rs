//! Error types for the ECH DoH H3 SDK

use thiserror::Error;
use uniffi::Error as UniffiError;

/// Result type alias
pub type Result<T> = std::result::Result<T, FetchError>;

/// Main error type for fetch operations
#[derive(Error, Debug, UniffiError)]
pub enum FetchError {
    /// Invalid URL format
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// DNS resolution failed
    #[error("DNS resolution failed: {0}")]
    DnsResolutionFailed(String),

    /// ECH configuration not found or invalid
    #[error("ECH config not found: {0}")]
    EchConfigNotFound(String),

    /// TLS handshake failed
    #[error("TLS handshake failed: {0}")]
    TlsHandshakeFailed(String),

    /// QUIC connection failed
    #[error("QUIC connection failed: {0}")]
    QuicConnectionFailed(String),

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),

    /// Timeout
    #[error("Timeout: {0}")]
    Timeout(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Engine already shutdown
    #[error("Engine already shutdown")]
    EngineShutdown,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Generic error
    #[error("{0}")]
    Generic(String),
}

impl From<anyhow::Error> for FetchError {
    fn from(err: anyhow::Error) -> Self {
        FetchError::Generic(err.to_string())
    }
}

impl From<std::io::Error> for FetchError {
    fn from(err: std::io::Error) -> Self {
        FetchError::IoError(err.to_string())
    }
}

impl From<url::ParseError> for FetchError {
    fn from(err: url::ParseError) -> Self {
        FetchError::InvalidUrl(err.to_string())
    }
}