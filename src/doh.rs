//! DoH (DNS over HTTPS) resolution with ECHConfig extraction
//!
//! This module handles:
//! - HTTPS record (TYPE 65) queries via DoH using a simple HTTP client
//! - ECHConfigList extraction from HTTPS records

use crate::error::{FetchError, Result};
use serde::Deserialize;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};
use url::Url;
use reqwest;

/// ECH configuration extracted from HTTPS records
#[derive(Debug, Clone)]
pub struct EchConfig {
    /// Raw ECHConfigList bytes
    pub config_list: Vec<u8>,
    /// Public key hash for verification
    pub public_key_hash: Vec<u8>,
    /// KEM ID
    pub kem_id: u16,
    /// Cipher suite IDs
    pub cipher_suites: Vec<u16>,
    /// Maximum name length
    pub max_name_len: u8,
    /// Public key length
    pub public_key_len: u16,
}

/// DNS response from DoH (JSON format)
#[derive(Debug, Deserialize)]
struct DohResponse {
    #[serde(rename = "Status")]
    status: u32,
    #[serde(rename = "Answer")]
    answer: Option<Vec<DohAnswer>>,
}

#[derive(Debug, Deserialize)]
struct DohAnswer {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "type")]
    record_type: u16,
    #[serde(rename = "TTL")]
    ttl: u32,
    #[serde(rename = "data")]
    data: String,
}

/// DoH resolver with ECHConfig extraction
pub struct DohResolver {
    doh_server_url: Url,
    bootstrap_ip: Option<IpAddr>,
    timeout: Duration,
    client: reqwest::blocking::Client,
}

impl DohResolver {
    /// Create a new DoH resolver
    pub fn new(
        doh_server: &str,
        bootstrap_ip: Option<String>,
        timeout_secs: u32,
    ) -> Result<Self> {
        let doh_server_url = Url::parse(doh_server)
            .map_err(|e| FetchError::InvalidConfig(format!("Invalid DoH server URL: {e}")))?;

        let bootstrap_ip = if let Some(ip_str) = bootstrap_ip {
            Some(IpAddr::from_str(&ip_str).map_err(|e| {
                FetchError::InvalidConfig(format!("Invalid bootstrap IP: {e}"))
            })?)
        } else {
            None
        };

        // Create reqwest client with native-tls
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout_secs as u64))
            .build()
            .map_err(|e| FetchError::IoError(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self {
            doh_server_url,
            bootstrap_ip,
            timeout: Duration::from_secs(timeout_secs as u64),
            client,
        })
    }

    /// Query HTTPS records (TYPE 65) for a domain and extract ECHConfig
    pub fn query_https_record(&self, domain: &str) -> Result<Option<EchConfig>> {
        debug!("Querying HTTPS record for domain: {}", domain);

        // Build DoH query URL (using JSON format)
        let query_url = format!(
            "{}?name={}&type=65&ct=application/dns-json",
            self.doh_server_url, domain
        );

        // Send query via DoH
        let resp = self.client.get(&query_url).send()
            .map_err(|e| FetchError::DnsResolutionFailed(format!("DoH query failed: {e}")))?;

        let doh_response: DohResponse = resp.json()
            .map_err(|e| FetchError::DnsResolutionFailed(format!("Failed to parse DoH response: {e}")))?;

        if doh_response.status != 0 {
            return Err(FetchError::DnsResolutionFailed(
                format!("DoH query failed with status: {}", doh_response.status)
            ));
        }

        // Extract HTTPS records from response
        if let Some(answers) = doh_response.answer {
            for answer in answers {
                if answer.record_type == 65 { // HTTPS record
                    if let Some(ech_config) = self.extract_ech_from_https_data(&answer.data)? {
                        info!("Found ECHConfig for {}", domain);
                        return Ok(Some(ech_config));
                    }
                }
            }
        }

        debug!("No ECHConfig found in HTTPS records for {}", domain);
        Ok(None)
    }

    /// Extract ECHConfig from HTTPS record data (base64 encoded)
    fn extract_ech_from_https_data(&self, data: &str) -> Result<Option<EchConfig>> {
        // The data field contains the raw HTTPS record in base64
        // Format: priority (u16) + target (domain) + svcparams...
        let decoded = base64::decode(data)
            .map_err(|e| FetchError::DnsResolutionFailed(format!("Failed to decode HTTPS record: {e}")))?;

        if decoded.len() < 4 {
            return Ok(None);
        }

        // Skip priority (2 bytes) and target (variable length, null-terminated or length-prefixed)
        // For simplicity, we'll look for ECH parameter (key=5) in the svcparams
        // This is a simplified parser - real implementation needs proper DNS wire format parsing
        
        // Search for ECH parameter (key=5) in the decoded data
        // SVCB param format: key (u16) + length (u16) + value
        let mut offset = 2; // skip priority
        
        // Skip target name (simplified - just find the svcparams section)
        // In practice, this needs proper DNS name parsing
        while offset < decoded.len() && decoded[offset] != 0 {
            offset += 1;
        }
        offset += 1; // skip null terminator
        
        // Now parse svcparams
        while offset + 3 < decoded.len() {
            let key = u16::from_be_bytes([decoded[offset], decoded[offset + 1]]);
            offset += 2;
            let len = u16::from_be_bytes([decoded[offset], decoded[offset + 1]]) as usize;
            offset += 2;
            
            if key == 5 { // ECH config
                if offset + len <= decoded.len() {
                    let ech_bytes = &decoded[offset..offset + len];
                    if let Some(parsed) = Self::parse_ech_config(ech_bytes)? {
                        return Ok(Some(parsed));
                    }
                }
            }
            
            offset += len;
        }
        
        Ok(None)
    }

    /// Parse ECHConfig from raw bytes
    fn parse_ech_config(config_list: &[u8]) -> Result<Option<EchConfig>> {
        if config_list.len() < 10 {
            return Ok(None);
        }

        let mut offset = 0;
        
        // Parse first ECHConfig entry
        // struct {
        //   uint16 cipher_suite;
        //   uint16 kem_id;
        //   uint8 max_name_len;
        //   uint16 public_key_len;
        //   opaque public_key[public_key_len];
        //   uint16 cipher_suites_len;
        //   CipherSuite cipher_suites[cipher_suites_len/2];
        // } ECHConfig;

        let cipher_suite = u16::from_be_bytes([config_list[offset], config_list[offset + 1]]);
        offset += 2;
        
        let kem_id = u16::from_be_bytes([config_list[offset], config_list[offset + 1]]);
        offset += 2;
        
        let max_name_len = config_list[offset];
        offset += 1;
        
        let public_key_len = u16::from_be_bytes([config_list[offset], config_list[offset + 1]]) as usize;
        offset += 2;
        
        if config_list.len() < offset + public_key_len + 2 {
            return Ok(None);
        }
        
        let public_key = config_list[offset..offset + public_key_len].to_vec();
        offset += public_key_len;
        
        let cipher_suites_len = u16::from_be_bytes([config_list[offset], config_list[offset + 1]]) as usize;
        offset += 2;
        
        let mut cipher_suites = Vec::new();
        for i in 0..cipher_suites_len / 2 {
            if offset + 1 < config_list.len() {
                let cs = u16::from_be_bytes([config_list[offset], config_list[offset + 1]]);
                cipher_suites.push(cs);
                offset += 2;
            }
        }

        // Compute public key hash (SHA-256)
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&public_key);
        let public_key_hash = hasher.finalize().to_vec();

        Ok(Some(EchConfig {
            config_list: config_list.to_vec(),
            public_key_hash,
            kem_id,
            cipher_suites,
            max_name_len,
            public_key_len: public_key_len as u16,
        }))
    }
}