//! HTTP/3 (QUIC) client implementation - simplified synchronous version

use crate::ech::{create_quiche_tls_config, TlsConfig};
use crate::error::{FetchError, Result};
use quiche::h3::{Config as H3Config, Connection as H3Connection, Header, NameValue};
use rand;
use std::net::UdpSocket;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use tracing::{info, warn};
use url::Url;

/// HTTP/3 client using quiche (blocking/synchronous)
pub struct H3Client {
    socket: UdpSocket,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    connection: quiche::Connection,
    h3_conn: H3Connection,
    request_timeout: Duration,
}

impl H3Client {
    /// Create a new HTTP/3 client
    pub fn connect(
        url: &Url,
        _tls_config: &TlsConfig,
        _request_timeout: Duration,
        connect_timeout: Duration,
    ) -> Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| FetchError::InvalidUrl("URL missing host".to_string()))?;

        let port = url.port().unwrap_or(443);

        // Resolve host to IP addresses
        let addrs = (host, port)
            .to_socket_addrs()
            .map_err(|e| FetchError::DnsResolutionFailed(format!("DNS resolution failed: {e}")))?;

        let peer_addr = addrs
            .into_iter()
            .next()
            .ok_or_else(|| FetchError::DnsResolutionFailed("No addresses resolved".to_string()))?;

        // Create UDP socket
        let local_addr = if peer_addr.is_ipv4() {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
        } else {
            SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
        };

        let socket = UdpSocket::bind(local_addr)
            .map_err(|e| FetchError::IoError(format!("Failed to bind UDP socket: {e}")))?;

        socket
            .connect(peer_addr)
            .map_err(|e| FetchError::IoError(format!("Failed to connect UDP socket: {e}")))?;

        let local_addr = socket
            .local_addr()
            .map_err(|e| FetchError::IoError(format!("Failed to get local addr: {e}")))?;

        // Create quiche config
        let mut config = create_quiche_tls_config(_tls_config)?;

        // Set up connection ID generation
        config.set_cc_algorithm(quiche::CongestionControlAlgorithm::CUBIC);

        // Create quiche connection
        let scid = quiche::ConnectionId::from_vec(rand::random::<[u8; 16]>().to_vec());
        let mut conn = quiche::connect(Some(host), &scid, local_addr, peer_addr, &mut config)
            .map_err(|e| {
                FetchError::QuicConnectionFailed(format!("Failed to create QUIC connection: {e}"))
            })?;

        // Perform handshake (blocking)
        Self::handshake_blocking(&socket, &mut conn, connect_timeout, peer_addr, local_addr)?;

        // Create HTTP/3 config and connection
        let h3_config = H3Config::new().map_err(|e| {
            FetchError::QuicConnectionFailed(format!("Failed to create H3 config: {e}"))
        })?;

        let h3_conn = H3Connection::with_transport(&mut conn, &h3_config).map_err(|e| {
            FetchError::QuicConnectionFailed(format!("Failed to create H3 connection: {e}"))
        })?;

        let client = Self {
            socket,
            local_addr,
            peer_addr,
            connection: conn,
            h3_conn,
            request_timeout: connect_timeout,
        };

        info!("HTTP/3 connection established to {}:{}", host, port);
        Ok(client)
    }

    /// Perform QUIC handshake (blocking)
    fn handshake_blocking(
        socket: &UdpSocket,
        conn: &mut quiche::Connection,
        timeout_dur: Duration,
        peer_addr: SocketAddr,
        local_addr: SocketAddr,
    ) -> Result<()> {
        let start = Instant::now();
        let mut buf = [0u8; 65535];

        loop {
            if start.elapsed() >= timeout_dur {
                return Err(FetchError::Timeout("QUIC handshake timeout".to_string()));
            }

            // Generate outgoing packets
            while let Ok((len, send_info)) = conn.send(&mut buf) {
                socket
                    .send_to(&buf[..len], send_info.to)
                    .map_err(|e| FetchError::IoError(format!("Failed to send QUIC packet: {e}")))?;
            }

            // Check if handshake is complete
            if conn.is_established() {
                break;
            }

            // Wait for incoming packets
            socket
                .set_read_timeout(Some(timeout_dur.saturating_sub(start.elapsed())))
                .map_err(|e| FetchError::IoError(format!("Failed to set read timeout: {e}")))?;

            match socket.recv_from(&mut buf) {
                Ok((len, from)) => {
                    if from != peer_addr {
                        continue;
                    }
                    let recv_info = quiche::RecvInfo {
                        from,
                        to: local_addr,
                    };
                    if let Err(e) = conn.recv(&mut buf[..len], recv_info) {
                        warn!("QUIC recv error: {}", e);
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    if conn.is_closed() {
                        return Err(FetchError::QuicConnectionFailed(
                            "QUIC connection closed during handshake".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    return Err(FetchError::IoError(format!("UDP recv error: {e}")));
                }
            }

            conn.on_timeout();
        }

        Ok(())
    }

    /// Send an HTTP request over HTTP/3
    pub fn send_request(
        &mut self,
        method: &str,
        path: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<Http3Response> {
        // Build request headers - quiche expects Header pairs
        let mut req_headers = vec![
            Header::new(b":method", method.as_bytes()),
            Header::new(b":path", path.as_bytes()),
            Header::new(b":scheme", b"https"),
            Header::new(b":authority", self.peer_addr.ip().to_string().as_bytes()),
        ];

        for (k, v) in headers {
            req_headers.push(Header::new(k.as_bytes(), v.as_bytes()));
        }

        // Send request
        let stream_id = self
            .h3_conn
            .send_request(&mut self.connection, &req_headers, body.is_empty())
            .map_err(|e| {
                FetchError::HttpRequestFailed(format!("Failed to send H3 request: {e}"))
            })?;

        // Send body if present
        if !body.is_empty() {
            self.h3_conn
                .send_body(&mut self.connection, stream_id, &body, true)
                .map_err(|e| {
                    FetchError::HttpRequestFailed(format!("Failed to send H3 body: {e}"))
                })?;
        }

        // Process events until response is complete
        let mut response_headers = Vec::new();
        let mut response_body = Vec::new();
        let mut finished = false;

        let start = Instant::now();
        let mut buf = [0u8; 65535];

        while !finished {
            if start.elapsed() >= self.request_timeout {
                return Err(FetchError::Timeout("HTTP/3 request timeout".to_string()));
            }

            // Process connection events
            while let Ok((len, send_info)) = self.connection.send(&mut buf) {
                self.socket
                    .send_to(&buf[..len], send_info.to)
                    .map_err(|e| FetchError::IoError(format!("Failed to send QUIC packet: {e}")))?;
            }

            // Poll for HTTP/3 events
            loop {
                match self.h3_conn.poll(&mut self.connection) {
                    Ok((_stream_id, event)) => {
                        match event {
                            quiche::h3::Event::Headers { list, .. } => {
                                response_headers = list
                                    .iter()
                                    .map(|h| {
                                        (
                                            String::from_utf8_lossy(NameValue::name(h)).to_string(),
                                            String::from_utf8_lossy(NameValue::value(h))
                                                .to_string(),
                                        )
                                    })
                                    .collect();
                            }
                            quiche::h3::Event::Data => {
                                // Read the data from the stream
                                let mut data_buf = vec![0u8; 65535];
                                match self.h3_conn.recv_body(
                                    &mut self.connection,
                                    _stream_id,
                                    &mut data_buf,
                                ) {
                                    Ok(len) => {
                                        response_body.extend_from_slice(&data_buf[..len]);
                                    }
                                    Err(quiche::h3::Error::Done) => {}
                                    Err(e) => {
                                        return Err(FetchError::HttpRequestFailed(format!(
                                            "H3 recv body error: {e}"
                                        )));
                                    }
                                }
                            }
                            quiche::h3::Event::Finished => {
                                finished = true;
                                break;
                            }
                            quiche::h3::Event::Reset(e) => {
                                return Err(FetchError::HttpRequestFailed(format!(
                                    "Stream reset: {}",
                                    e
                                )));
                            }
                            quiche::h3::Event::PriorityUpdate => {}
                            quiche::h3::Event::GoAway => {}
                        }
                    }
                    Err(quiche::h3::Error::Done) => break,
                    Err(e) => {
                        return Err(FetchError::HttpRequestFailed(format!("H3 poll error: {e}")));
                    }
                }
            }

            if finished {
                break;
            }

            // Wait for incoming packets
            self.socket
                .set_read_timeout(Some(self.request_timeout.saturating_sub(start.elapsed())))
                .map_err(|e| FetchError::IoError(format!("Failed to set read timeout: {e}")))?;

            match self.socket.recv_from(&mut buf) {
                Ok((len, from)) => {
                    if from != self.peer_addr {
                        continue;
                    }
                    let recv_info = quiche::RecvInfo {
                        from,
                        to: self.local_addr,
                    };
                    if let Err(e) = self.connection.recv(&mut buf[..len], recv_info) {
                        warn!("QUIC recv error: {}", e);
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    if self.connection.is_closed() {
                        return Err(FetchError::QuicConnectionFailed(
                            "QUIC connection closed".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    return Err(FetchError::IoError(format!("UDP recv error: {e}")));
                }
            }

            self.connection.on_timeout();
        }

        // Extract status code from headers
        let status_code = response_headers
            .iter()
            .find(|(k, _)| k == ":status")
            .map(|(_, v)| v.parse::<u16>().unwrap_or(0))
            .unwrap_or(0);

        // Convert headers to map, skipping pseudo-headers
        let headers: std::collections::HashMap<String, String> = response_headers
            .into_iter()
            .filter(|(k, _)| !k.starts_with(':'))
            .collect();

        Ok(Http3Response {
            status_code,
            headers,
            body: response_body,
        })
    }

    /// Close the connection gracefully
    pub fn close(&mut self) -> Result<()> {
        // Send CONNECTION_CLOSE frame
        let _ = self.connection.close(true, 0, b"");

        // Flush any remaining packets
        let mut buf = [0u8; 65535];
        while let Ok((len, send_info)) = self.connection.send(&mut buf) {
            let _ = self.socket.send_to(&buf[..len], send_info.to);
        }
        Ok(())
    }
}

/// HTTP/3 response
#[derive(Debug, Clone)]
pub struct Http3Response {
    /// HTTP status code (e.g., 200, 404, 500)
    pub status_code: u16,
    /// Response headers as key-value pairs
    pub headers: std::collections::HashMap<String, String>,
    /// Response body as raw bytes
    pub body: Vec<u8>,
}
