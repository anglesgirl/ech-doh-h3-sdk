//! ech-doh-h3-sdk: Cross-platform DoH + TLS 1.3 ECH + HTTP/3 Network SDK
//!
//! This crate provides a unified interface for making HTTP requests with:
//! - DNS over HTTPS (DoH) for secure DNS resolution
//! - TLS 1.3 Encrypted Client Hello (ECH) for SNI protection
//! - HTTP/3 (QUIC) with fallback to HTTP/2 over TLS 1.3

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::all)]

pub mod config;
pub mod doh;
pub mod ech;
pub mod engine;
pub mod error;
pub mod h2;
pub mod h3;

use uniffi::Object;

pub use config::EngineConfig;
pub use error::{FetchError, Result};
pub use engine::Engine;

// Re-export UniFFI types
pub use crate::engine::HttpMethod;
pub use crate::engine::HttpResponse;

uniffi::setup_scaffolding!();