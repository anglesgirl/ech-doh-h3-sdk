//! Configuration types for the ECH DoH H3 Engine

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uniffi::Record;

/// Configuration for the ECH DoH H3 Engine
#[derive(Debug, Clone, Record, Serialize, Deserialize)]
pub struct EngineConfig {
    /// DoH server URL (e.g., "https://1.1.1.1/dns-query" or "https://dns.google/dns-query")
    pub doh_server: String,

    /// Optional custom DoH server IP for bootstrapping (bypasses system DNS)
    pub doh_bootstrap_ip: Option<String>,

    /// Connection timeout in seconds
    pub connect_timeout_secs: u32,

    /// Request timeout in seconds
    pub request_timeout_secs: u32,

    /// Enable HTTP/3 (QUIC) - if false, uses TCP + TLS 1.3 only
    pub enable_h3: bool,

    /// Enable ECH - if false, uses standard TLS without ECH
    pub enable_ech: bool,

    /// User-Agent string
    pub user_agent: String,

    /// Verify TLS certificates (set false only for testing)
    pub verify_certificates: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            doh_server: "https://1.1.1.1/dns-query".to_string(),
            doh_bootstrap_ip: None,
            connect_timeout_secs: 10,
            request_timeout_secs: 30,
            enable_h3: true,
            enable_ech: true,
            user_agent: "ech-doh-h3-sdk/0.1.0".to_string(),
            verify_certificates: true,
        }
    }
}

impl EngineConfig {
    /// Get connection timeout as Duration
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs as u64)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs as u64)
    }

    /// Validate configuration
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.doh_server.is_empty() {
            return Err(crate::error::FetchError::InvalidConfig(
                "DoH server URL cannot be empty".to_string(),
            ));
        }

        if self.connect_timeout_secs == 0 {
            return Err(crate::error::FetchError::InvalidConfig(
                "Connection timeout must be > 0".to_string(),
            ));
        }

        if self.request_timeout_secs == 0 {
            return Err(crate::error::FetchError::InvalidConfig(
                "Request timeout must be > 0".to_string(),
            ));
        }

        // Validate DoH server URL
        url::Url::parse(&self.doh_server).map_err(|e| {
            crate::error::FetchError::InvalidConfig(format!("Invalid DoH server URL: {e}"))
        })?;

        Ok(())
    }
}