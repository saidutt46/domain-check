//! WHOIS protocol implementation for domain availability checking.
//!
//! This module provides WHOIS-based domain checking as a fallback when RDAP is not available.
//! WHOIS is the traditional protocol for domain registration data, though it provides
//! unstructured text responses that require parsing.

use crate::error::DomainCheckError;
use crate::types::{DomainResult, CheckMethod};
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
        let result = tokio::time::timeout(
            self.timeout,
            self.execute_whois_command(domain)
        ).await;

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
            Err(_) => {
                Err(DomainCheckError::timeout("WHOIS query", self.timeout))
            }
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
                    format!("Failed to execute whois command: {}. Make sure 'whois' is installed.", e)
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
                    DomainCheckError::whois(
                        domain,
                        format!("Failed to execute whois retry: {}", e)
                    )
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
            if whois_output.contains(pattern) {
                return Ok(true);
            }
        }

        // Check for taken patterns
        let taken_pattern_count = taken_patterns.iter()
            .filter(|pattern| whois_output.contains(*pattern))
            .count();

        // If we found multiple "taken" indicators, the domain is likely taken
        if taken_pattern_count >= 2 {
            return Ok(false);
        }

        // If the output is very short, it might indicate availability
        if whois_output.trim().len() < 50 {
            return Ok(true);
        }

        // Default to taken if we can't determine (conservative approach)
        // This is safer than incorrectly reporting an available domain
        Ok(false)
    }

    /// Check if the WHOIS output indicates rate limiting.
    fn is_rate_limited(&self, output: &str) -> bool {
        let rate_limit_patterns = [
            "rate limit exceeded",
            "too many requests",
            "try again later",
            "quota exceeded",
            "limit exceeded",
            "throttled",
            "blocked",
            "rate-limited",
        ];

        rate_limit_patterns.iter().any(|pattern| output.contains(pattern))
    }
}

impl Default for WhoisClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if the system has a working whois command.
///
/// This function can be used to verify that WHOIS functionality is available
/// before attempting to use the WhoisClient.
///
/// # Returns
///
/// `true` if the whois command is available and working, `false` otherwise.
pub async fn is_whois_available() -> bool {
    match Command::new("whois")
        .arg("--version")
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => {
            // Try with a different flag that's more universal
            match Command::new("whois")
                .arg("example.com")
                .output()
                .await
            {
                Ok(_) => true,
                Err(_) => false,
            }
        }
    }
}

/// Get the version of the system's whois command.
///
/// This is useful for debugging and ensuring compatibility.
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

    #[test]
    fn test_parse_availability_patterns() {
        let client = WhoisClient::new();

        // Test available patterns
        let available_text = "No matching record found for example-not-registered.com";
        assert_eq!(client.parse_whois_availability(available_text).unwrap(), true);

        let available_text2 = "Domain not found";
        assert_eq!(client.parse_whois_availability(available_text2).unwrap(), true);

        // Test taken patterns
        let taken_text = "Domain Status: clientTransferProhibited\nRegistrar: Example Registrar\nCreation Date: 2020-01-01";
        assert_eq!(client.parse_whois_availability(taken_text).unwrap(), false);
    }

    #[test]
    fn test_rate_limit_detection() {
        let client = WhoisClient::new();

        assert_eq!(client.is_rate_limited("Rate limit exceeded. Try again later."), true);
        assert_eq!(client.is_rate_limited("Too many requests from your IP."), true);
        assert_eq!(client.is_rate_limited("Normal whois response"), false);
    }

    #[test]
    fn test_whois_client_creation() {
        let client = WhoisClient::new();
        assert_eq!(client.timeout, Duration::from_secs(5));

        let custom_client = WhoisClient::with_timeout(Duration::from_secs(10));
        assert_eq!(custom_client.timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_whois_availability_check() {
        // This test only runs if whois is available
        if is_whois_available().await {
            let client = WhoisClient::new();
            // Test with a domain that should exist
            let result = client.check_domain("google.com").await;
            
            // We don't assert the specific result since it depends on the system,
            // but we verify that the function completes without error
            assert!(result.is_ok());
        }
    }
}