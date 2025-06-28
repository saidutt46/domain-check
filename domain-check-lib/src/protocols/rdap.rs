//! RDAP (Registration Data Access Protocol) implementation.
//!
//! This module provides functionality to check domain availability using the RDAP protocol,
//! which is the modern replacement for WHOIS. RDAP provides structured JSON responses
//! with standardized data formats.

use crate::error::DomainCheckError;
use crate::protocols::registry::{extract_tld, get_rdap_endpoint};
use crate::types::{CheckMethod, DomainInfo, DomainResult};
use reqwest::StatusCode;
use std::time::{Duration, Instant};

/// RDAP client for checking domain availability.
///
/// This client handles RDAP protocol communication, including endpoint discovery,
/// request formatting, response parsing, and error handling.
#[derive(Clone)]
pub struct RdapClient {
    /// HTTP client for making RDAP requests
    http_client: reqwest::Client,
    /// Timeout for RDAP requests
    timeout: Duration,
    /// Whether to use IANA bootstrap for unknown TLDs
    use_bootstrap: bool,
}

impl RdapClient {
    /// Create a new RDAP client with default settings.
    pub fn new() -> Result<Self, DomainCheckError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| {
                DomainCheckError::network_with_source(
                    "Failed to create RDAP HTTP client",
                    e.to_string(),
                )
            })?;

        Ok(Self {
            http_client,
            timeout: Duration::from_secs(3),
            use_bootstrap: false,
        })
    }

    /// Create a new RDAP client with custom settings.
    pub fn with_config(timeout: Duration, use_bootstrap: bool) -> Result<Self, DomainCheckError> {
        let http_client = reqwest::Client::builder()
            .timeout(timeout + Duration::from_secs(2)) // Add buffer for HTTP timeout
            .build()
            .map_err(|e| {
                DomainCheckError::network_with_source(
                    "Failed to create RDAP HTTP client",
                    e.to_string(),
                )
            })?;

        Ok(Self {
            http_client,
            timeout,
            use_bootstrap,
        })
    }

    /// Check domain availability using RDAP.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain name to check (e.g., "example.com")
    ///
    /// # Returns
    ///
    /// A `DomainResult` with availability status and optional registration details.
    ///
    /// # Errors
    ///
    /// Returns `DomainCheckError` if:
    /// - The domain format is invalid
    /// - No RDAP endpoint is available for the TLD
    /// - Network errors occur
    /// - The RDAP response cannot be parsed
    pub async fn check_domain(&self, domain: &str) -> Result<DomainResult, DomainCheckError> {
        let start_time = Instant::now();

        // Extract TLD and get RDAP endpoint
        let tld = extract_tld(domain)?;
        let endpoint = get_rdap_endpoint(&tld, self.use_bootstrap).await?;

        // Build RDAP URL
        let rdap_url = format!("{}{}", endpoint, domain);

        // 🔍 DEBUG: Log the URL being requested
        if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
            println!("🔍 Attempting RDAP request to: {}", rdap_url);
        }

        // Make RDAP request with timeout
        let result =
            tokio::time::timeout(self.timeout, self.make_rdap_request(&rdap_url, domain)).await;

        let check_duration = start_time.elapsed();

        match result {
            Ok(Ok((available, info))) => Ok(DomainResult {
                domain: domain.to_string(),
                available: Some(available),
                info,
                check_duration: Some(check_duration),
                method_used: if self.use_bootstrap {
                    CheckMethod::Bootstrap
                } else {
                    CheckMethod::Rdap
                },
                error_message: None,
            }),
            Ok(Err(e)) => {
                // 🔍 DEBUG: Log RDAP errors
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 RDAP Error for {}: {}", domain, e);
                }

                // Check if the error indicates the domain is available
                if e.indicates_available() {
                    Ok(DomainResult {
                        domain: domain.to_string(),
                        available: Some(true),
                        info: None,
                        check_duration: Some(check_duration),
                        method_used: CheckMethod::Rdap,
                        error_message: None,
                    })
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                // 🔍 DEBUG: Log timeout
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 RDAP Timeout for {} after {:?}", domain, self.timeout);
                }

                Err(DomainCheckError::timeout("RDAP request", self.timeout))
            }
        }
    }

    /// Make an RDAP request to the specified URL.
    /// Make an RDAP request to the specified URL.
    async fn make_rdap_request(
        &self,
        rdap_url: &str,
        domain: &str,
    ) -> Result<(bool, Option<DomainInfo>), DomainCheckError> {
        // First attempt
        let response = self.http_client.get(rdap_url).send().await.map_err(|e| {
            // 🔍 DEBUG: Log request errors
            if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                println!("🔍 HTTP Request failed for {}: {}", rdap_url, e);
                if e.is_timeout() {
                    println!("   └─ Timeout error");
                } else if e.is_connect() {
                    println!("   └─ Connection error");
                } else if e.is_request() {
                    println!("   └─ Request error");
                }
            }
            DomainCheckError::rdap(domain, format!("Request failed: {}", e))
        })?;

        // 🔍 DEBUG: Log response status
        if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
            println!("🔍 HTTP Response for {}: {}", domain, response.status());
        }

        match response.status() {
            StatusCode::OK => {
                // Domain exists, parse the response
                let json = response.json::<serde_json::Value>().await.map_err(|e| {
                    DomainCheckError::rdap(domain, format!("Failed to parse JSON: {}", e))
                })?;

                // 🔍 DEBUG: Print the actual JSON response for analysis
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 RDAP Response for {}:", domain);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json).unwrap_or_default()
                    );
                    println!("--- End RDAP Response ---\n");
                }

                let domain_info = extract_domain_info(&json);

                // 🔍 DEBUG: Print extracted info
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 Extracted Info for {}:", domain);
                    println!("  Registrar: {:?}", domain_info.registrar);
                    println!("  Created: {:?}", domain_info.creation_date);
                    println!("  Expires: {:?}", domain_info.expiration_date);
                    println!("  Status: {:?}", domain_info.status);
                    println!("--- End Extracted Info ---\n");
                }

                Ok((false, Some(domain_info)))
            }
            StatusCode::NOT_FOUND => {
                // Domain is available
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 Domain {} is available (404)", domain);
                }
                Ok((true, None))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                // Rate limited, try once more after a short delay
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 Rate limited for {}, retrying after 500ms...", domain);
                }

                tokio::time::sleep(Duration::from_millis(500)).await;

                let retry_response = self.http_client.get(rdap_url).send().await.map_err(|e| {
                    DomainCheckError::rdap(domain, format!("Retry request failed: {}", e))
                })?;

                match retry_response.status() {
                    StatusCode::OK => {
                        let json =
                            retry_response
                                .json::<serde_json::Value>()
                                .await
                                .map_err(|e| {
                                    DomainCheckError::rdap(
                                        domain,
                                        format!("Failed to parse retry JSON: {}", e),
                                    )
                                })?;

                        let domain_info = extract_domain_info(&json);
                        Ok((false, Some(domain_info)))
                    }
                    StatusCode::NOT_FOUND => Ok((true, None)),
                    code => {
                        if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                            println!("🔍 Retry failed for {} with status: {}", domain, code);
                        }
                        Err(DomainCheckError::rdap_with_status(
                            domain,
                            format!("RDAP server error after retry: {}", code),
                            code.as_u16(),
                        ))
                    }
                }
            }
            code => {
                if std::env::var("DOMAIN_CHECK_DEBUG_RDAP").is_ok() {
                    println!("🔍 RDAP server error for {} with status: {}", domain, code);
                }
                Err(DomainCheckError::rdap_with_status(
                    domain,
                    format!("RDAP server returned error: {}", code),
                    code.as_u16(),
                ))
            }
        }
    }
}

impl Default for RdapClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default RDAP client")
    }
}

/// Extract domain information from an RDAP JSON response.
///
/// This function parses the standardized RDAP JSON format and extracts
/// relevant domain registration details.
///
/// # Arguments
///
/// * `json` - The RDAP JSON response
///
/// # Returns
///
/// A `DomainInfo` struct with extracted registration details.
pub fn extract_domain_info(json: &serde_json::Value) -> DomainInfo {
    let mut info = DomainInfo::default();

    // Extract registrar information from entities
    if let Some(entities) = json.get("entities").and_then(|e| e.as_array()) {
        for entity in entities {
            if let Some(roles) = entity.get("roles").and_then(|r| r.as_array()) {
                let is_registrar = roles.iter().any(|role| role.as_str() == Some("registrar"));

                if is_registrar {
                    // Try to get registrar name from vcardArray
                    if let Some(name) = extract_vcard_name(entity) {
                        info.registrar = Some(name);
                        break;
                    }
                    // Fallback to publicIds or handle
                    else if let Some(name) = extract_entity_identifier(entity) {
                        info.registrar = Some(name);
                        break;
                    }
                }
            }
        }
    }

    // Extract dates from events
    if let Some(events) = json.get("events").and_then(|e| e.as_array()) {
        for event in events {
            if let (Some(event_action), Some(event_date)) = (
                event.get("eventAction").and_then(|a| a.as_str()),
                event.get("eventDate").and_then(|d| d.as_str()),
            ) {
                match event_action {
                    "registration" => info.creation_date = Some(event_date.to_string()),
                    "expiration" => info.expiration_date = Some(event_date.to_string()),
                    "last update of RDAP database" | "last changed" => {
                        info.updated_date = Some(event_date.to_string())
                    }
                    _ => {}
                }
            }
        }
    }

    // Extract status codes
    if let Some(statuses) = json.get("status").and_then(|s| s.as_array()) {
        for status in statuses {
            if let Some(status_str) = status.as_str() {
                info.status.push(status_str.to_string());
            }
        }
    }

    // Extract nameservers
    if let Some(nameservers) = json.get("nameservers").and_then(|ns| ns.as_array()) {
        for nameserver in nameservers {
            if let Some(ldh_name) = nameserver.get("ldhName").and_then(|name| name.as_str()) {
                info.nameservers.push(ldh_name.to_string());
            }
        }
    }

    info
}

/// Extract organization name from vCard format in RDAP entity.
fn extract_vcard_name(entity: &serde_json::Value) -> Option<String> {
    entity
        .get("vcardArray")
        .and_then(|v| v.as_array())
        .and_then(|a| a.get(1))
        .and_then(|a| a.as_array())
        .and_then(|items| {
            for item in items {
                if let Some(item_array) = item.as_array() {
                    if item_array.len() >= 4 {
                        if let Some(first) = item_array.first().and_then(|f| f.as_str()) {
                            if first == "fn" {
                                return item_array
                                    .get(3)
                                    .and_then(|n| n.as_str())
                                    .map(String::from);
                            }
                        }
                    }
                }
            }
            None
        })
}

/// Extract entity identifier from publicIds or handle.
fn extract_entity_identifier(entity: &serde_json::Value) -> Option<String> {
    // Try publicIds first
    if let Some(public_ids) = entity.get("publicIds").and_then(|p| p.as_array()) {
        if let Some(id) = public_ids
            .first()
            .and_then(|id| id.get("identifier"))
            .and_then(|i| i.as_str())
        {
            return Some(id.to_string());
        }
    }

    // Fallback to handle
    if let Some(handle) = entity.get("handle").and_then(|h| h.as_str()) {
        return Some(handle.to_string());
    }

    // Fallback to name
    entity
        .get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rdap_client_creation() {
        let client = RdapClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_extract_domain_info_basic() {
        let json = serde_json::json!({
            "events": [
                {
                    "eventAction": "registration",
                    "eventDate": "1995-08-14T04:00:00Z"
                },
                {
                    "eventAction": "expiration",
                    "eventDate": "2025-08-13T04:00:00Z"
                }
            ],
            "status": ["client delete prohibited", "client transfer prohibited"]
        });

        let info = extract_domain_info(&json);
        assert_eq!(info.creation_date, Some("1995-08-14T04:00:00Z".to_string()));
        assert_eq!(
            info.expiration_date,
            Some("2025-08-13T04:00:00Z".to_string())
        );
        assert_eq!(info.status.len(), 2);
    }

    #[test]
    fn test_extract_vcard_name() {
        let entity = serde_json::json!({
            "vcardArray": [
                "vcard",
                [
                    ["fn", {}, "text", "Example Registrar Inc."]
                ]
            ]
        });

        let name = extract_vcard_name(&entity);
        assert_eq!(name, Some("Example Registrar Inc.".to_string()));
    }
}
