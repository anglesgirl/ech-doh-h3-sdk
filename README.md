# ech-doh-h3-sdk

Cross-platform DoH + TLS 1.3 ECH + HTTP/3 Network SDK for Android and iOS.

## Features

- **DNS over HTTPS (DoH)** - Secure DNS resolution with custom endpoints (Cloudflare, Google, etc.)
- **TLS 1.3 ECH (Encrypted Client Hello)** - Hides SNI from network observers
- **HTTP/3 (QUIC)** - Modern transport protocol with 0-RTT, improved performance
- **Automatic Fallback** - Seamless fallback from HTTP/3 to HTTP/1.1 over TLS 1.3
- **Binary-Safe** - All body data handled as `Vec<u8>` (no C-string limitations)
- **Cross-Platform** - Native Kotlin (Android) and Swift (iOS) bindings via UniFFI
- **Zero-Config** - Works out of the box with sensible defaults

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      UniFFI Interface                        │
├─────────────────────────────────────────────────────────────┤
│  Engine (Main Entry Point)                                   │
│  ├── fetch() - Main request method                           │
│  ├── get() / post() - Convenience methods                    │
│  └── shutdown() - Cleanup                                    │
├─────────────────────────────────────────────────────────────┤
│  DoH Resolver (dns-over-https-rustls)                       │
│  └── Queries HTTPS records (TYPE 65) for ECHConfig          │
├─────────────────────────────────────────────────────────────┤
│  TLS Config (rustls + webpki-roots)                         │
│  └── TLS 1.3 with certificate verification                  │
├─────────────────────────────────────────────────────────────┤
│  HTTP/3 Client (quiche + boringssl-vendored)                │
│  └── QUIC + HTTP/3 with ECH support                         │
├─────────────────────────────────────────────────────────────┤
│  HTTP Fallback (reqwest + native-tls)                       │
│  └── TCP + TLS 1.3 when QUIC fails                          │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Rust

```toml
# Cargo.toml
[dependencies]
ech-doh-h3-sdk = { git = "https://github.com/your-org/ech-doh-h3-sdk" }
```

```rust
use ech_doh_h3_sdk::{Engine, EngineConfig, HttpMethod};

let config = EngineConfig::default();
let engine = Engine::new(config).unwrap();

let response = engine.get(
    "https://example.com/api/data".to_string(),
    std::collections::HashMap::new()
).unwrap();

println!("Status: {}", response.status_code);
println!("Body: {}", String::from_utf8_lossy(&response.body));
```

### Android (Kotlin)

```kotlin
// Add to build.gradle
implementation("com.example:ech-doh-h3-sdk:0.1.0@aar")

// Usage
val config = EngineConfig(
    dohServer = "https://1.1.1.1/dns-query",
    dohBootstrapIp = null,
    connectTimeoutSecs = 10,
    requestTimeoutSecs = 30,
    enableH3 = true,
    enableEch = true,
    userAgent = "MyApp/1.0",
    verifyCertificates = true
)

val engine = Engine.new(config)

val response = engine.fetch(
    url = "https://example.com/api/data",
    method = HttpMethod.GET,
    headers = mapOf("Accept" to "application/json"),
    body = byteArrayOf()
)
```

### iOS (Swift)

```swift
// Add ech_doh_h3_sdk.xcframework to your Xcode project

let config = EngineConfig(
    dohServer: "https://1.1.1.1/dns-query",
    dohBootstrapIp: nil,
    connectTimeoutSecs: 10,
    requestTimeoutSecs: 30,
    enableH3: true,
    enableEch: true,
    userAgent: "MyApp/1.0",
    verifyCertificates: true
)

let engine = Engine.new(config: config)!

let response = try await engine.fetch(
    url: "https://example.com/api/data",
    method: .GET,
    headers: ["Accept": "application/json"],
    body: Data()
)
```

## Building

### Prerequisites

- Rust 1.85+ (MSRV)
- Android NDK r25+ (for Android builds)
- Xcode 15+ (for iOS builds)

### Build Commands

```bash
# Install dependencies
./build.sh deps

# Generate UniFFI bindings
./build.sh bindings

# Build for Android
./build.sh android

# Build for iOS
./build.sh ios

# Build everything
./build.sh all
```

### Android Output

- `target/android/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libech_doh_h3_sdk.so`
- `target/android/aar/` - AAR package

### iOS Output

- `target/ios/ech_doh_h3_sdk.xcframework` - Universal XCFramework

## Configuration

```rust
pub struct EngineConfig {
    /// DoH server URL
    pub doh_server: String,                    // Default: "https://1.1.1.1/dns-query"
    
    /// Optional bootstrap IP for DoH server (bypasses system DNS)
    pub doh_bootstrap_ip: Option<String>,      // Default: None
    
    /// Connection timeout (seconds)
    pub connect_timeout_secs: u32,             // Default: 10
    
    /// Request timeout (seconds)
    pub request_timeout_secs: u32,             // Default: 30
    
    /// Enable HTTP/3 (QUIC)
    pub enable_h3: bool,                       // Default: true
    
    /// Enable ECH
    pub enable_ech: bool,                      // Default: true
    
    /// User-Agent string
    pub user_agent: String,                    // Default: "ech-doh-h3-sdk/0.1.0"
    
    /// Verify TLS certificates
    pub verify_certificates: bool,             // Default: true
}
```

## API

### Engine Methods

```rust
impl Engine {
    /// Create new engine instance
    pub fn new(config: EngineConfig) -> Result<Arc<Self>>
    
    /// Generic fetch request
    pub fn fetch(
        &self,
        url: String,
        method: HttpMethod,
        headers: HashMap<String, String>,
        body: Vec<u8>
    ) -> Result<HttpResponse>
    
    /// Convenience GET
    pub fn get(&self, url: String, headers: HashMap<String, String>) -> Result<HttpResponse>
    
    /// Convenience POST
    pub fn post(
        &self,
        url: String,
        headers: HashMap<String, String>,
        body: Vec<u8>
    ) -> Result<HttpResponse>
    
    /// Shutdown engine
    pub fn shutdown(&self)
}
```

### HttpMethod Enum

```rust
pub enum HttpMethod {
    GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
}
```

### HttpResponse

```rust
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}
```

## Error Handling

```rust
pub enum FetchError {
    InvalidUrl(String),
    InvalidConfig(String),
    DnsResolutionFailed(String),
    TlsHandshakeFailed(String),
    QuicConnectionFailed(String),
    HttpRequestFailed(String),
    IoError(String),
    Timeout(String),
    EngineShutdown,
    Generic(String),
}
```

## Security Considerations

- **Fail-Closed**: ECH failures result in connection failure (no SNI exposure)
- **Certificate Verification**: Enabled by default, disable only for testing
- **No Auto-Retry**: Prevents amplification attacks on upstream services
- **Binary-Safe**: Body handling uses `Vec<u8>` (supports images, compressed data, protobuf)

## Performance

- HTTP/3 0-RTT for repeated connections
- Connection pooling via quiche
- Minimal allocations in hot path
- LTO + opt-level=z for release builds

## License

MIT OR Apache-2.0