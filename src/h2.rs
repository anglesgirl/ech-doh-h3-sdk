//! HTTP fallback client using reqwest with rustls

use crate::ech::TlsConfig;
use crate::error::{FetchError, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// HTTP fallback client using reqwest
pub struct HttpFallbackClient {
    client: reqwest::blocking::Client,
}

impl HttpFallbackClient {
    /// Create a new HTTP fallback client
    pub fn new(
        _tls_config: Arc<TlsConfig>,
        _connect_timeout: Duration,
        _request_timeout: Duration,
    ) -> Result<Self> {
        // reqwest with rustls-tls handles TLS internally
        let client = reqwest::blocking::Client::builder()
            .timeout(_request_timeout)
            .connect_timeout(_connect_timeout)
            .build()
            .map_err(|e| FetchError::IoError(format!("Failed to create HTTP client: {e}")))?;

        info!("HTTP fallback client created (using reqwest with rustls-tls)");

        Ok(Self { client })
    }

    /// Perform an HTTP request
    pub fn send_request(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<HttpFallbackResponse> {
        let mut req = self.client.request(
            method
                .parse()
                .map_err(|e| FetchError::HttpRequestFailed(format!("Invalid HTTP method: {e}")))?,
            url,
        );

        for (key, value) in headers {
            req = req.header(key, value);
        }

        if !body.is_empty() {
            req = req.body(body);
        }

        let response = req
            .send()
            .map_err(|e| FetchError::HttpRequestFailed(format!("HTTP request failed: {e}")))?;

        let status_code = response.status().as_u16();
        let headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
            .collect();

        let body = response
            .bytes()
            .map_err(|e| FetchError::HttpRequestFailed(format!("Failed to read response: {e}")))?
            .to_vec();

        Ok(HttpFallbackResponse {
            status_code,
            headers,
            body,
        })
    }
}

/// HTTP fallback response
#[derive(Debug, Clone)]
pub struct HttpFallbackResponse {
    /// HTTP status code (e.g., 200, 404, 500)
    pub status_code: u16,
    /// Response headers as key-value pairs
    pub headers: std::collections::HashMap<String, String>,
    /// Response body as raw bytes
    pub body: Vec<u8>,
}
