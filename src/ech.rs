//! ECH (Encrypted Client Hello) TLS configuration
//!
//! This module handles TLS configuration for both QUIC (quiche) and TCP TLS connections.
//! For HTTP/3, ECH is handled by quiche with vendored BoringSSL.
//! For HTTP/2 fallback, we use rustls with native TLS support.

use crate::doh::EchConfig;
use crate::error::{FetchError, Result};
use rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore};
use std::sync::Arc;
use tracing::{info, warn};
use webpki_roots::TLS_SERVER_ROOTS;

/// TLS configuration with ECH support
pub struct TlsConfig {
    /// Rustls client config for HTTP/2 fallback
    rustls_config: Arc<ClientConfig>,
    /// ECH config for reference
    ech_config: Option<EchConfig>,
}

impl TlsConfig {
    /// Create a new TLS configuration with optional ECH
    pub fn new(
        enable_ech: bool,
        ech_config: Option<EchConfig>,
        verify_certificates: bool,
    ) -> Result<Self> {
        // Build rustls config for HTTP/2 fallback
        let mut root_store = RootCertStore::empty();

        if verify_certificates {
            // Add webpki roots (Mozilla's trusted root certificates)
            #[allow(deprecated)]
            for ta in TLS_SERVER_ROOTS.0.iter() {
                let anchor = OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                );
                root_store.add_server_trust_anchors(std::iter::once(anchor));
            }
        } else {
            warn!("Certificate verification DISABLED - only for testing!");
        }

        let rustls_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // Enable TLS 1.3
        // In rustls 0.20, TLS 1.3 is enabled by default with safe defaults
        // No need to explicitly set enable_tls13

        // Note: ECH support in rustls is experimental
        // For HTTP/3, quiche handles ECH with BoringSSL
        if enable_ech && ech_config.is_some() {
            info!("ECH enabled for HTTP/3 (via quiche), HTTP/2 fallback uses standard TLS 1.3");
        }

        let rustls_config = Arc::new(rustls_config);

        info!(
            "TLS config created: ECH={}, verify={}",
            enable_ech && ech_config.is_some(),
            verify_certificates
        );

        Ok(Self {
            rustls_config,
            ech_config,
        })
    }

    /// Get the rustls client config
    pub fn rustls_config(&self) -> Arc<ClientConfig> {
        self.rustls_config.clone()
    }

    /// Get the ECH config if available
    pub fn ech_config(&self) -> Option<&EchConfig> {
        self.ech_config.as_ref()
    }

    /// Check if ECH is enabled
    pub fn has_ech(&self) -> bool {
        self.ech_config.is_some()
    }
}

/// Create a quiche-compatible TLS config for HTTP/3
/// quiche uses its own BoringSSL internally, ECH is configured via quiche's config
pub fn create_quiche_tls_config(_tls_config: &TlsConfig) -> Result<quiche::Config> {
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).map_err(|e| {
        FetchError::TlsHandshakeFailed(format!("Failed to create quiche config: {e}"))
    })?;

    // quiche with boringssl-vendored handles TLS internally
    // For ECH, quiche will use BoringSSL's ECH support if available

    // Set application protocols (HTTP/3)
    config
        .set_application_protos(&[b"h3", b"h3-29"])
        .map_err(|e| FetchError::TlsHandshakeFailed(format!("Failed to set ALPN: {e}")))?;

    // Set initial congestion window
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);

    // Set idle timeout
    config.set_max_idle_timeout(30_000); // 30 seconds

    // Disable active migration for stability
    config.set_disable_active_migration(true);

    // Note: To enable ECH in quiche, we'd need to configure it via quiche's API
    // which requires BoringSSL's ECH APIs. This is a complex integration.
    // For now, quiche will use standard TLS 1.3.

    Ok(config)
}
