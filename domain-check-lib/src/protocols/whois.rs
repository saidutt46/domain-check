//! WHOIS protocol implementation for domain availability checking.
//!
//! This module provides WHOIS-based domain checking as a fallback when RDAP is not available.
//! WHOIS is the traditional protocol for domain registration data, though it provides
//! unstructured text responses that require parsing.

use crate::error::DomainCheckError;
use crate::types::{CheckMethod, DomainResult};
use std::time::{Duration, Instant};
use tokio::process::Command;

/// WHOIS client for checking domain availability using the system's whois command.
///
/// This client uses the system's `whois` command-line tool to query domain information.
/// It's designed as a fallback when RDAP is not available or fails.
#[derive(Clone)]
pub struct WhoisClient {
    /// Timeout for WHOIS requests
    timeout: Duration,
}

impl WhoisClient {
    /// Create a new WHOIS client with default settings.
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }

    /// Create a new WHOIS client with custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Check domain availability using WHOIS.
    ///
    /// This method executes the system's `whois` command and parses the output
    /// to determine if a domain is available or taken.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain name to check (e.g., "example.com")
    ///
    /// # Returns
    ///
    /// A `DomainResult` with availability status. Note that WHOIS typically
    /// doesn't provide structured registration details like RDAP.
    ///
    /// # Errors
    ///
    /// Returns `DomainCheckError` if:
    /// - The `whois` command is not available on the system
    /// - The WHOIS query times out
    /// - The WHOIS response cannot be parsed
    pub async fn check_domain(&self, domain: &str) -> Result<DomainResult, DomainCheckError> {
        let start_time = Instant::now();

        // Execute WHOIS command with timeout
        let result = tokio::time::timeout(self.timeout, self.execute_whois_command(domain)).await;

        let check_duration = start_time.elapsed();

        match result {
            Ok(Ok(available)) => {
                Ok(DomainResult {
                    domain: domain.to_string(),
                    available: Some(available),
                    info: None, // WHOIS parsing for detailed info is complex and inconsistent
                    check_duration: Some(check_duration),
                    method_used: CheckMethod::Whois,
                    error_message: None,
                })
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(DomainCheckError::timeout("WHOIS query", self.timeout)),
        }
    }

    /// Check domain availability using WHOIS with a specific server.
    ///
    /// This method uses `whois -h <server> <domain>` for a targeted query,
    /// falling back to bare `whois <domain>` if the `-h` flag fails.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain name to check (e.g., "example.com")
    /// * `server` - The WHOIS server hostname (e.g., "whois.verisign-grs.com")
    pub async fn check_domain_with_server(
        &self,
        domain: &str,
        server: &str,
    ) -> Result<DomainResult, DomainCheckError> {
        let start_time = Instant::now();

        let result = tokio::time::timeout(
            self.timeout,
            self.execute_whois_command_with_server(domain, server),
        )
        .await;

        let check_duration = start_time.elapsed();

        match result {
            Ok(Ok(available)) => Ok(DomainResult {
                domain: domain.to_string(),
                available: Some(available),
                info: None,
                check_duration: Some(check_duration),
                method_used: CheckMethod::Whois,
                error_message: None,
            }),
            Ok(Err(_)) => {
                // Targeted query failed, fall back to bare whois
                self.check_domain(domain).await
            }
            Err(_) => Err(DomainCheckError::timeout("WHOIS query", self.timeout)),
        }
    }

    /// Execute the system whois command and parse the result.
    async fn execute_whois_command(&self, domain: &str) -> Result<bool, DomainCheckError> {
        // First attempt
        let output = Command::new("whois")
            .arg(domain)
            .output()
            .await
            .map_err(|e| {
                DomainCheckError::whois(
                    domain,
                    format!(
                        "Failed to execute whois command: {}. Make sure 'whois' is installed.",
                        e
                    ),
                )
            })?;

        let output_text = String::from_utf8_lossy(&output.stdout).to_lowercase();

        // Check for rate limiting first
        if self.is_rate_limited(&output_text) {
            // Wait and retry once
            tokio::time::sleep(Duration::from_millis(1000)).await;

            let retry_output = Command::new("whois")
                .arg(domain)
                .output()
                .await
                .map_err(|e| {
                    DomainCheckError::whois(domain, format!("Failed to execute whois retry: {}", e))
                })?;

            let retry_text = String::from_utf8_lossy(&retry_output.stdout).to_lowercase();
            self.parse_whois_availability(&retry_text)
        } else {
            self.parse_whois_availability(&output_text)
        }
    }

    /// Execute whois command with a specific server (-h flag).
    async fn execute_whois_command_with_server(
        &self,
        domain: &str,
        server: &str,
    ) -> Result<bool, DomainCheckError> {
        let output = Command::new("whois")
            .arg("-h")
            .arg(server)
            .arg(domain)
            .output()
            .await
            .map_err(|e| {
                DomainCheckError::whois(
                    domain,
                    format!("Failed to execute whois -h {} command: {}", server, e),
                )
            })?;

        let output_text = String::from_utf8_lossy(&output.stdout).to_lowercase();

        if self.is_rate_limited(&output_text) {
            tokio::time::sleep(Duration::from_millis(1000)).await;

            let retry_output = Command::new("whois")
                .arg("-h")
                .arg(server)
                .arg(domain)
                .output()
                .await
                .map_err(|e| {
                    DomainCheckError::whois(domain, format!("Failed to execute whois retry: {}", e))
                })?;

            let retry_text = String::from_utf8_lossy(&retry_output.stdout).to_lowercase();
            self.parse_whois_availability(&retry_text)
        } else {
            self.parse_whois_availability(&output_text)
        }
    }

    /// Parse WHOIS output to determine domain availability.
    ///
    /// This function looks for common patterns in WHOIS responses that indicate
    /// whether a domain is available or taken. WHOIS responses vary significantly
    /// between registries, so this uses a comprehensive set of patterns.
    fn parse_whois_availability(&self, whois_output: &str) -> Result<bool, DomainCheckError> {
        let output_lower = whois_output.to_lowercase();

        // First check for invalid TLD or server errors
        let invalid_tld_patterns = [
            "no whois server is known",
            "no whois server",
            "invalid tld",
            "unknown tld",
            "tld not found",
            "no such tld",
            "bad tld",
            "invalid domain extension",
        ];

        for pattern in &invalid_tld_patterns {
            if output_lower.contains(pattern) {
                return Err(DomainCheckError::bootstrap(
                    "unknown",
                    "Invalid or unsupported TLD for WHOIS lookup",
                ));
            }
        }

        // Patterns that typically indicate domain availability
        let available_patterns = [
            "no match",
            "not found",
            "no data found",
            "no entries found",
            "domain not found",
            "domain available",
            "status: available",
            "status: free",
            "no information available",
            "not registered",
            "no matching record",
            "domain status: no object found",
            "the queried object does not exist",
            "object does not exist",
            "no matching entry",
            "domain name not found",
            "this domain name has not been registered",
            "no found",
        ];

        // Patterns that indicate the domain is definitely taken
        let taken_patterns = [
            "domain status:",
            "registrar:",
            "creation date:",
            "created:",
            "registry domain id:",
            "registrant:",
            "admin contact:",
            "tech contact:",
            "name server:",
            "nameservers:",
            "expiry date:",
            "expires:",
            "updated:",
            "last updated:",
        ];

        // Check for availability patterns first (more specific)
        for pattern in &available_patterns {
            if output_lower.contains(pattern) {
                return Ok(true);
            }
        }

        // Check for taken patterns
        let taken_pattern_count = taken_patterns
            .iter()
            .filter(|pattern| output_lower.contains(*pattern))
            .count();

        // If we found multiple "taken" indicators, the domain is likely taken
        if taken_pattern_count >= 2 {
            return Ok(false);
        }

        // If the output is very short, it might indicate availability
        if output_lower.trim().len() < 50 {
            return Ok(true);
        }

        // For truly ambiguous cases, return an error instead of guessing
        // This prevents false positives for invalid domains
        Err(DomainCheckError::whois(
            "unknown",
            "Unable to determine domain status from WHOIS response",
        ))
    }

    /// Check if the WHOIS output indicates rate limiting.
    fn is_rate_limited(&self, output: &str) -> bool {
        let output_lower = output.to_lowercase();
        let rate_limit_patterns = [
            "rate limit exceeded",
            "too many requests",
            "try again later",
            "quota exceeded",
            "limit exceeded",
            "throttled",
            "blocked",
            "rate-limited",
            "too many requests from your ip",
        ];

        rate_limit_patterns
            .iter()
            .any(|pattern| output_lower.contains(pattern))
    }
}

impl Default for WhoisClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Discover the authoritative WHOIS server for a TLD via IANA referral.
///
/// Uses the system `whois` command to query `whois.iana.org` for the TLD,
/// then parses the response for a `refer:` line containing the authoritative
/// WHOIS server hostname.
///
/// # Arguments
///
/// * `tld` - The TLD to look up (e.g., "com", "co", "museum")
///
/// # Returns
///
/// The WHOIS server hostname (e.g., "whois.verisign-grs.com"), or None if
/// no referral was found or the query failed.
pub async fn discover_whois_server(tld: &str) -> Option<String> {
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let output = Command::new("whois")
            .arg("-h")
            .arg("whois.iana.org")
            .arg(tld)
            .output()
            .await
            .ok()?;

        let response = String::from_utf8_lossy(&output.stdout);
        parse_iana_refer_response(&response)
    })
    .await;

    result.unwrap_or(None)
}

/// Parse an IANA WHOIS response for the authoritative WHOIS server.
///
/// The IANA WHOIS response may use either `refer:` or `whois:` to indicate
/// the authoritative WHOIS server for a TLD. We check both fields, preferring
/// `refer:` when present.
///
/// ```text
/// whois:        whois.verisign-grs.com
/// refer:        whois.verisign-grs.com
/// ```
fn parse_iana_refer_response(response: &str) -> Option<String> {
    let mut whois_server = None;

    for line in response.lines() {
        let line_trimmed = line.trim();
        if let Some(server) = line_trimmed.strip_prefix("refer:") {
            let server = server.trim();
            if !server.is_empty() {
                // `refer:` is the canonical field — return immediately
                return Some(server.to_string());
            }
        } else if let Some(server) = line_trimmed.strip_prefix("whois:") {
            let server = server.trim();
            if !server.is_empty() {
                whois_server = Some(server.to_string());
            }
        }
    }

    whois_server
}

/// Check if the system has a working whois command.
///
/// This function can be used to verify that WHOIS functionality is available
/// before attempting to use the WhoisClient.
///
/// # Returns
///
/// `true` if the whois command is available and working, `false` otherwise.
#[allow(dead_code)]
pub async fn is_whois_available() -> bool {
    match Command::new("whois").arg("--version").output().await {
        Ok(output) => output.status.success(),
        Err(_) => {
            // Try with a different flag that's more universal
            (Command::new("whois").arg("example.com").output().await).is_ok()
        }
    }
}

/// Get the version of the system's whois command.
///
/// This is useful for debugging and ensuring compatibility.
#[allow(dead_code)]
pub async fn get_whois_version() -> Result<String, DomainCheckError> {
    let output = Command::new("whois")
        .arg("--version")
        .output()
        .await
        .map_err(|e| {
            DomainCheckError::whois("version", format!("Failed to get whois version: {}", e))
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("Unknown whois version".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── WhoisClient creation ────────────────────────────────────────────

    #[test]
    fn test_whois_client_new() {
        let client = WhoisClient::new();
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_whois_client_with_timeout() {
        let client = WhoisClient::with_timeout(Duration::from_secs(10));
        assert_eq!(client.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_whois_client_default() {
        let client = WhoisClient::default();
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    // ── parse_whois_availability: available patterns ────────────────────

    #[test]
    fn test_available_no_match() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("No match for domain")
            .unwrap());
    }

    #[test]
    fn test_available_not_found() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("Not found: example.com")
            .unwrap());
    }

    #[test]
    fn test_available_domain_not_found() {
        let client = WhoisClient::new();
        assert!(client.parse_whois_availability("Domain not found").unwrap());
    }

    #[test]
    fn test_available_no_data_found() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("No data found for this query")
            .unwrap());
    }

    #[test]
    fn test_available_no_entries_found() {
        let client = WhoisClient::new();
        assert!(client.parse_whois_availability("No entries found").unwrap());
    }

    #[test]
    fn test_available_domain_available() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("Domain available for registration")
            .unwrap());
    }

    #[test]
    fn test_available_status_free() {
        let client = WhoisClient::new();
        assert!(client.parse_whois_availability("Status: free").unwrap());
    }

    #[test]
    fn test_available_not_registered() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("This domain is not registered")
            .unwrap());
    }

    #[test]
    fn test_available_object_does_not_exist() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("The queried object does not exist")
            .unwrap());
    }

    #[test]
    fn test_available_no_found() {
        let client = WhoisClient::new();
        assert!(client.parse_whois_availability("No found").unwrap());
    }

    #[test]
    fn test_available_case_insensitive() {
        let client = WhoisClient::new();
        assert!(client
            .parse_whois_availability("NO MATCH FOR DOMAIN")
            .unwrap());
        assert!(client.parse_whois_availability("DOMAIN NOT FOUND").unwrap());
    }

    // ── parse_whois_availability: taken patterns ────────────────────────

    #[test]
    fn test_taken_multiple_indicators() {
        let client = WhoisClient::new();
        let taken = "Domain Status: clientTransferProhibited\nRegistrar: GoDaddy\nCreation Date: 2020-01-01";
        assert!(!client.parse_whois_availability(taken).unwrap());
    }

    #[test]
    fn test_taken_registrar_and_nameserver() {
        let client = WhoisClient::new();
        let taken = "Registrar: MarkMonitor Inc.\nName Server: ns1.google.com";
        assert!(!client.parse_whois_availability(taken).unwrap());
    }

    #[test]
    fn test_taken_created_and_expires() {
        let client = WhoisClient::new();
        let taken = "Created: 2015-01-01\nExpires: 2025-01-01";
        assert!(!client.parse_whois_availability(taken).unwrap());
    }

    #[test]
    fn test_taken_single_indicator_not_enough() {
        let client = WhoisClient::new();
        // Only one "taken" pattern — needs >= 2 to confirm taken
        let ambiguous = "Registrar: SomeRegistrar\nSome other random text that is long enough to exceed fifty characters";
        // Single taken indicator + long text = error (ambiguous)
        let result = client.parse_whois_availability(ambiguous);
        assert!(result.is_err());
    }

    // ── parse_whois_availability: invalid TLD patterns ──────────────────

    #[test]
    fn test_invalid_tld_no_whois_server() {
        let client = WhoisClient::new();
        let result = client.parse_whois_availability("No whois server is known for this TLD");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_tld_unknown() {
        let client = WhoisClient::new();
        let result = client.parse_whois_availability("Unknown TLD: .fakext");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_tld_bad() {
        let client = WhoisClient::new();
        let result = client.parse_whois_availability("Bad TLD specified in query");
        assert!(result.is_err());
    }

    // ── parse_whois_availability: short output = available ──────────────

    #[test]
    fn test_short_output_considered_available() {
        let client = WhoisClient::new();
        // < 50 chars, no patterns matched = available
        assert!(client.parse_whois_availability("Some short text").unwrap());
    }

    #[test]
    fn test_empty_output_considered_available() {
        let client = WhoisClient::new();
        assert!(client.parse_whois_availability("").unwrap());
    }

    // ── parse_whois_availability: ambiguous = error ─────────────────────

    #[test]
    fn test_ambiguous_output_returns_error() {
        let client = WhoisClient::new();
        // Long text, no available or taken patterns matched with >= 2 hits
        let ambiguous = "This is some random whois response that doesn't match any known pattern and is longer than fifty characters total";
        let result = client.parse_whois_availability(ambiguous);
        assert!(result.is_err());
        // Display renders as "WHOIS lookup failed" for generic errors
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("WHOIS lookup failed"));
    }

    // ── is_rate_limited ─────────────────────────────────────────────────

    #[test]
    fn test_rate_limited_patterns() {
        let client = WhoisClient::new();
        assert!(client.is_rate_limited("Rate limit exceeded. Try again later."));
        assert!(client.is_rate_limited("Too many requests from your IP."));
        assert!(client.is_rate_limited("Quota exceeded for this connection"));
        assert!(client.is_rate_limited("Request throttled, please wait"));
        assert!(client.is_rate_limited("Your IP has been blocked"));
        assert!(client.is_rate_limited("You have been rate-limited"));
    }

    #[test]
    fn test_rate_limited_case_insensitive() {
        let client = WhoisClient::new();
        assert!(client.is_rate_limited("RATE LIMIT EXCEEDED"));
        assert!(client.is_rate_limited("Too Many Requests"));
    }

    #[test]
    fn test_not_rate_limited() {
        let client = WhoisClient::new();
        assert!(!client.is_rate_limited("Normal whois response"));
        assert!(!client.is_rate_limited("Domain Status: active"));
        assert!(!client.is_rate_limited(""));
    }

    // ── parse_iana_refer_response ───────────────────────────────────────

    #[test]
    fn test_iana_refer_standard() {
        let response =
            "% IANA WHOIS server\n\nrefer:        whois.verisign-grs.com\n\ndomain:       COM\n";
        assert_eq!(
            parse_iana_refer_response(response),
            Some("whois.verisign-grs.com".to_string())
        );
    }

    #[test]
    fn test_iana_refer_none() {
        let response = "% IANA WHOIS server\ndomain: TEST\nstatus: ACTIVE\n";
        assert_eq!(parse_iana_refer_response(response), None);
    }

    #[test]
    fn test_iana_refer_empty_value() {
        let response = "refer:        \ndomain: COM\n";
        assert_eq!(parse_iana_refer_response(response), None);
    }

    #[test]
    fn test_iana_whois_field_fallback() {
        let response = "whois:        whois.verisign-grs.com\ndomain: COM\n";
        assert_eq!(
            parse_iana_refer_response(response),
            Some("whois.verisign-grs.com".to_string())
        );
    }

    #[test]
    fn test_iana_refer_takes_precedence_over_whois() {
        let response =
            "whois:        whois.old-server.com\nrefer:        whois.correct-server.com\n";
        assert_eq!(
            parse_iana_refer_response(response),
            Some("whois.correct-server.com".to_string())
        );
    }

    #[test]
    fn test_iana_empty_whois_field() {
        let response = "whois:        \ndomain: COM\n";
        assert_eq!(parse_iana_refer_response(response), None);
    }

    #[test]
    fn test_iana_empty_response() {
        assert_eq!(parse_iana_refer_response(""), None);
    }

    // ── Network-dependent test ──────────────────────────────────────────

    #[tokio::test]
    async fn test_whois_availability_check() {
        if is_whois_available().await {
            let client = WhoisClient::new();
            let result = client.check_domain("google.com").await;
            assert!(result.is_ok());
        }
    }
}
