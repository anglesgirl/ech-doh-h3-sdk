//! Main Engine implementation - ties together DoH, ECH, H3, and HTTP fallback

use crate::config::EngineConfig;
use crate::doh::DohResolver;
use crate::ech::TlsConfig;
use crate::error::{FetchError, Result};
use crate::h2::HttpFallbackClient;
use crate::h3::H3Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use url::Url;

/// HTTP method enumeration for UniFFI
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HttpMethod {
    /// HTTP GET method
    GET,
    /// HTTP POST method
    POST,
    /// HTTP PUT method
    PUT,
    /// HTTP DELETE method
    DELETE,
    /// HTTP PATCH method
    PATCH,
    /// HTTP HEAD method
    HEAD,
    /// HTTP OPTIONS method
    OPTIONS,
}

/// HTTP response structure for UniFFI
#[derive(Debug, Clone, uniffi::Record)]
pub struct HttpResponse {
    /// HTTP status code (e.g., 200, 404, 500)
    pub status_code: u16,
    /// Response headers as key-value pairs
    pub headers: HashMap<String, String>,
    /// Response body as raw bytes
    pub body: Vec<u8>,
}

/// Main Engine for HTTP requests with DoH + ECH + H3/HTTP fallback
#[derive(uniffi::Object)]
pub struct Engine {
    config: EngineConfig,
    doh_resolver: Mutex<Option<Arc<DohResolver>>>,
    shutdown: Mutex<bool>,
}

impl Engine {
    /// Create a new Engine with the given configuration
    #[uniffi::constructor]
    pub fn new(config: EngineConfig) -> Result<Arc<Self>> {
        config.validate()?;

        let engine = Arc::new(Self {
            config,
            doh_resolver: Mutex::new(None),
            shutdown: Mutex::new(false),
        });

        Ok(engine)
    }

    /// Perform an HTTP request
    pub fn fetch(
        &self,
        url: String,
        method: crate::HttpMethod,
        headers: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<crate::HttpResponse> {
        // Check shutdown
        if *self.shutdown.lock().unwrap() {
            return Err(FetchError::EngineShutdown);
        }

        // Run async fetch in a blocking manner for UniFFI sync interface
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| FetchError::Generic(format!("Failed to create runtime: {e}")))?;

        rt.block_on(self.fetch_async(url, method, headers, body))
    }

    /// Async fetch implementation
    async fn fetch_async(
        &self,
        url: String,
        method: crate::HttpMethod,
        headers: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<crate::HttpResponse> {
        let url = Url::parse(&url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;

        let host = url
            .host_str()
            .ok_or_else(|| FetchError::InvalidUrl("URL missing host".to_string()))?;

        let method_str = match method {
            crate::HttpMethod::GET => "GET",
            crate::HttpMethod::POST => "POST",
            crate::HttpMethod::PUT => "PUT",
            crate::HttpMethod::DELETE => "DELETE",
            crate::HttpMethod::PATCH => "PATCH",
            crate::HttpMethod::HEAD => "HEAD",
            crate::HttpMethod::OPTIONS => "OPTIONS",
        };

        // Convert headers
        let header_vec: Vec<(String, String)> = headers.into_iter().collect();

        // Try HTTP/3 first if enabled
        if self.config.enable_h3 {
            match self.fetch_h3(&url, method_str, header_vec.clone(), body.clone()) {
                Ok(response) => {
                    info!("HTTP/3 request succeeded");
                    return Ok(response.into());
                }
                Err(e) => {
                    warn!("HTTP/3 failed, falling back to HTTP: {}", e);
                }
            }
        }

        // Fallback to HTTP over TLS 1.3
        let response = self.fetch_fallback(&url, method_str, header_vec, body)?;
        Ok(response.into())
    }

    /// Fetch via HTTP/3
    fn fetch_h3(
        &self,
        url: &Url,
        method: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<crate::HttpResponse> {
        // Get or create DoH resolver
        let doh_resolver = self.get_doh_resolver()?;

        // Query ECH config for this domain
        let host = url.host_str().unwrap();
        let ech_config = doh_resolver.query_https_record(host)?;

        // Create TLS config with ECH
        let tls_config = TlsConfig::new(
            self.config.enable_ech,
            ech_config,
            self.config.verify_certificates,
        )?;

        // Connect via HTTP/3
        let mut h3_client = H3Client::connect(
            url,
            &tls_config,
            self.config.request_timeout(),
            self.config.connect_timeout(),
        )?;

        // Send request
        let h3_response = h3_client.send_request(method, url.path(), headers, body)?;

        // Close connection
        let _ = h3_client.close();

        Ok(crate::HttpResponse {
            status_code: h3_response.status_code,
            headers: h3_response.headers,
            body: h3_response.body,
        })
    }

    /// Fetch via HTTP fallback (reqwest)
    fn fetch_fallback(
        &self,
        url: &Url,
        method: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<crate::HttpResponse> {
        // Get or create DoH resolver
        let doh_resolver = self.get_doh_resolver()?;

        // Query ECH config for this domain
        let _host = url.host_str().unwrap();
        let ech_config = doh_resolver.query_https_record(_host)?;

        // Create TLS config with ECH
        let tls_config = Arc::new(TlsConfig::new(
            self.config.enable_ech,
            ech_config,
            self.config.verify_certificates,
        )?);

        // Create HTTP fallback client
        let http_client = HttpFallbackClient::new(
            tls_config,
            self.config.connect_timeout(),
            self.config.request_timeout(),
        )?;

        // Send request
        let http_response = http_client.send_request(method, url.as_ref(), headers, body)?;

        Ok(crate::HttpResponse {
            status_code: http_response.status_code,
            headers: http_response.headers,
            body: http_response.body,
        })
    }

    /// Get or create DoH resolver (lazy initialization)
    fn get_doh_resolver(&self) -> Result<Arc<DohResolver>> {
        // Check if already cached
        if let Some(resolver) = self.doh_resolver.lock().unwrap().as_ref() {
            return Ok(resolver.clone());
        }

        // Create new resolver
        let resolver = DohResolver::new(
            &self.config.doh_server,
            self.config.doh_bootstrap_ip.clone(),
            self.config.connect_timeout_secs,
        )?;

        let resolver = Arc::new(resolver);

        // Cache it
        *self.doh_resolver.lock().unwrap() = Some(resolver.clone());

        Ok(resolver)
    }

    /// Convenience GET method
    pub fn get(
        &self,
        url: String,
        headers: HashMap<String, String>,
    ) -> Result<crate::HttpResponse> {
        self.fetch(url, crate::HttpMethod::GET, headers, Vec::new())
    }

    /// Convenience POST method
    pub fn post(
        &self,
        url: String,
        headers: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<crate::HttpResponse> {
        self.fetch(url, crate::HttpMethod::POST, headers, body)
    }

    /// Shutdown the engine
    pub fn shutdown(&self) {
        *self.shutdown.lock().unwrap() = true;
    }
}

// Conversion from H3 response to UniFFI HttpResponse
impl From<crate::h3::Http3Response> for crate::HttpResponse {
    fn from(r: crate::h3::Http3Response) -> Self {
        Self {
            status_code: r.status_code,
            headers: r.headers,
            body: r.body,
        }
    }
}

// Conversion from HTTP fallback response to UniFFI HttpResponse
impl From<crate::h2::HttpFallbackResponse> for crate::HttpResponse {
    fn from(r: crate::h2::HttpFallbackResponse) -> Self {
        Self {
            status_code: r.status_code,
            headers: r.headers,
            body: r.body,
        }
    }
}
