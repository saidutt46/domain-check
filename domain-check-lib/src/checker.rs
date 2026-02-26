//! Main domain checker implementation.
//!
//! This module provides the primary `DomainChecker` struct that orchestrates
//! domain availability checking using RDAP, WHOIS, and bootstrap protocols.

use crate::error::DomainCheckError;
use crate::protocols::registry::{extract_tld, get_whois_server};
use crate::protocols::{RdapClient, WhoisClient};
use crate::types::{CheckConfig, CheckMethod, DomainResult};
use crate::utils::validate_domain;
use futures_util::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Check a single domain using the provided clients (for concurrent processing).
///
/// This is a helper function that implements the same logic as `check_domain`
/// but works with cloned client instances for concurrent execution.
async fn check_single_domain_concurrent(
    domain: &str,
    rdap_client: &RdapClient,
    whois_client: &WhoisClient,
    config: &CheckConfig,
) -> Result<DomainResult, DomainCheckError> {
    // Validate domain format first
    validate_domain(domain)?;

    // Try RDAP first
    match rdap_client.check_domain(domain).await {
        Ok(result) => {
            // RDAP succeeded, filter info based on configuration
            let mut filtered_result = result;
            if !config.detailed_info {
                filtered_result.info = None;
            }
            Ok(filtered_result)
        }
        Err(rdap_error) => {
            // RDAP failed, try WHOIS fallback if enabled
            if config.enable_whois_fallback {
                // Discover WHOIS server for targeted query
                let whois_result = whois_with_discovery(domain, whois_client).await;

                match whois_result {
                    Ok(whois_result) => {
                        let mut filtered_result = whois_result;
                        if !config.detailed_info {
                            filtered_result.info = None;
                        }
                        Ok(filtered_result)
                    }
                    Err(whois_error) => {
                        // Both RDAP and WHOIS failed, determine best response

                        // Check if either error indicates the domain is available
                        if rdap_error.indicates_available() || whois_error.indicates_available() {
                            Ok(DomainResult {
                                domain: domain.to_string(),
                                available: Some(true),
                                info: None,
                                check_duration: None,
                                method_used: CheckMethod::Rdap,
                                error_message: None,
                            })
                        }
                        // Check if it's an unknown TLD or truly ambiguous case
                        else if matches!(rdap_error, DomainCheckError::BootstrapError { .. })
                            || matches!(whois_error, DomainCheckError::BootstrapError { .. })
                            || whois_error
                                .to_string()
                                .contains("Unable to determine domain status")
                        {
                            // Return unknown status for invalid TLDs or ambiguous cases
                            Ok(DomainResult {
                                domain: domain.to_string(),
                                available: None, // Unknown status
                                info: None,
                                check_duration: None,
                                method_used: CheckMethod::Unknown,
                                error_message: Some(
                                    "Unknown TLD or unable to determine status".to_string(),
                                ),
                            })
                        } else {
                            // Return the RDAP error as it's usually more informative
                            Err(rdap_error)
                        }
                    }
                }
            } else {
                // No fallback enabled, return RDAP error
                Err(rdap_error)
            }
        }
    }
}

/// Perform WHOIS check with server discovery for targeted queries.
///
/// If the TLD's authoritative WHOIS server can be discovered via IANA referral,
/// uses `whois -h <server> <domain>` for a more reliable query. Falls back to
/// bare `whois <domain>` otherwise.
async fn whois_with_discovery(
    domain: &str,
    whois_client: &WhoisClient,
) -> Result<DomainResult, DomainCheckError> {
    let tld = extract_tld(domain).ok();
    let whois_server = if let Some(ref t) = tld {
        get_whois_server(t).await
    } else {
        None
    };

    if let Some(server) = whois_server {
        whois_client.check_domain_with_server(domain, &server).await
    } else {
        whois_client.check_domain(domain).await
    }
}

/// Main domain checker that coordinates availability checking operations.
///
/// The `DomainChecker` handles all aspects of domain checking including:
/// - Protocol selection (RDAP vs WHOIS)
/// - Concurrent processing
/// - Error handling and retries
/// - Result formatting
///
/// # Example
///
/// ```rust,no_run
/// use domain_check_lib::{DomainChecker, CheckConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let checker = DomainChecker::new();
///     let result = checker.check_domain("example.com").await?;
///     println!("Available: {:?}", result.available);
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct DomainChecker {
    /// Configuration settings for this checker instance
    config: CheckConfig,
    /// RDAP client for modern domain checking
    rdap_client: RdapClient,
    /// WHOIS client for fallback domain checking
    whois_client: WhoisClient,
}

impl DomainChecker {
    /// Create a new domain checker with default configuration.
    ///
    /// Default settings:
    /// - Concurrency: 20
    /// - Timeout: 5 seconds
    /// - WHOIS fallback: enabled
    /// - Bootstrap: enabled
    /// - Detailed info: disabled
    pub fn new() -> Self {
        let config = CheckConfig::default();
        let rdap_client = RdapClient::with_config(config.rdap_timeout, config.enable_bootstrap)
            .expect("Failed to create RDAP client");
        let whois_client = WhoisClient::with_timeout(config.whois_timeout);

        Self {
            config,
            rdap_client,
            whois_client,
        }
    }

    /// Create a new domain checker with custom configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use domain_check_lib::{DomainChecker, CheckConfig};
    /// use std::time::Duration;
    ///
    /// let config = CheckConfig::default()
    ///     .with_concurrency(20)
    ///     .with_timeout(Duration::from_secs(10))
    ///     .with_detailed_info(true);
    ///     
    /// let checker = DomainChecker::with_config(config);
    /// ```
    pub fn with_config(config: CheckConfig) -> Self {
        let rdap_client = RdapClient::with_config(config.rdap_timeout, config.enable_bootstrap)
            .expect("Failed to create RDAP client");
        let whois_client = WhoisClient::with_timeout(config.whois_timeout);

        Self {
            config,
            rdap_client,
            whois_client,
        }
    }

    /// Check availability of a single domain.
    ///
    /// This is the most basic operation - check one domain and return the result.
    /// The domain should be a fully qualified domain name (e.g., "example.com").
    ///
    /// The checking process:
    /// 1. Validates the domain format
    /// 2. Attempts RDAP check first (modern protocol)
    /// 3. Falls back to WHOIS if RDAP fails and fallback is enabled
    /// 4. Returns comprehensive result with timing and method information
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain name to check (e.g., "example.com")
    ///
    /// # Returns
    ///
    /// A `DomainResult` containing availability status and optional details.
    ///
    /// # Errors
    ///
    /// Returns `DomainCheckError` if:
    /// - The domain name is invalid
    /// - Network errors occur
    /// - All checking methods fail
    pub async fn check_domain(&self, domain: &str) -> Result<DomainResult, DomainCheckError> {
        // Validate domain format first
        validate_domain(domain)?;

        // Try RDAP first
        match self.rdap_client.check_domain(domain).await {
            Ok(result) => {
                // RDAP succeeded, filter info based on configuration
                Ok(self.filter_result_info(result))
            }
            Err(rdap_error) => {
                // RDAP failed, try WHOIS fallback if enabled
                if self.config.enable_whois_fallback {
                    // Use WHOIS with server discovery for targeted queries
                    match whois_with_discovery(domain, &self.whois_client).await {
                        Ok(whois_result) => Ok(self.filter_result_info(whois_result)),
                        Err(whois_error) => {
                            // Both RDAP and WHOIS failed, determine best response

                            // Check if either error indicates the domain is available
                            if rdap_error.indicates_available() || whois_error.indicates_available()
                            {
                                Ok(DomainResult {
                                    domain: domain.to_string(),
                                    available: Some(true),
                                    info: None,
                                    check_duration: None,
                                    method_used: CheckMethod::Rdap,
                                    error_message: None,
                                })
                            }
                            // Check if it's an unknown TLD or truly ambiguous case
                            else if matches!(rdap_error, DomainCheckError::BootstrapError { .. })
                                || matches!(whois_error, DomainCheckError::BootstrapError { .. })
                                || whois_error
                                    .to_string()
                                    .contains("Unable to determine domain status")
                            {
                                // Return unknown status for invalid TLDs or ambiguous cases
                                Ok(DomainResult {
                                    domain: domain.to_string(),
                                    available: None, // Unknown status
                                    info: None,
                                    check_duration: None,
                                    method_used: CheckMethod::Unknown,
                                    error_message: Some(
                                        "Unknown TLD or unable to determine status".to_string(),
                                    ),
                                })
                            } else {
                                // Return the most informative error
                                Err(rdap_error)
                            }
                        }
                    }
                } else {
                    // No fallback enabled, return RDAP error
                    Err(rdap_error)
                }
            }
        }
    }

    /// Filter domain result info based on configuration.
    ///
    /// If detailed_info is disabled, removes the info field to keep results clean.
    fn filter_result_info(&self, mut result: DomainResult) -> DomainResult {
        if !self.config.detailed_info {
            result.info = None;
        }
        result
    }

    /// Check availability of multiple domains concurrently.
    ///
    /// This method processes all domains in parallel according to the
    /// concurrency setting, then returns all results at once.
    ///
    /// # Arguments
    ///
    /// * `domains` - Slice of domain names to check
    ///
    /// # Returns
    ///
    /// Vector of `DomainResult` in the same order as input domains.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use domain_check_lib::DomainChecker;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let checker = DomainChecker::new();
    ///     let domains = vec!["example.com".to_string(), "google.com".to_string(), "test.org".to_string()];
    ///     let results = checker.check_domains(&domains).await?;
    ///     
    ///     for result in results {
    ///         println!("{}: {:?}", result.domain, result.available);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_domains(
        &self,
        domains: &[String],
    ) -> Result<Vec<DomainResult>, DomainCheckError> {
        if domains.is_empty() {
            return Ok(Vec::new());
        }

        // Create semaphore to limit concurrent operations
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));
        let mut handles = Vec::new();

        // Spawn concurrent tasks for each domain
        for (index, domain) in domains.iter().enumerate() {
            let domain = domain.clone();
            let semaphore = Arc::clone(&semaphore);

            // Clone the checker components we need
            let rdap_client = self.rdap_client.clone();
            let whois_client = self.whois_client.clone();
            let config = self.config.clone();

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await.unwrap();

                // Check this domain
                let result =
                    check_single_domain_concurrent(&domain, &rdap_client, &whois_client, &config)
                        .await;

                // Return with original index to maintain order
                (index, result)
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete and collect results
        let mut indexed_results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((index, result)) => indexed_results.push((index, result)),
                Err(e) => {
                    return Err(DomainCheckError::internal(format!(
                        "Concurrent task failed: {}",
                        e
                    )));
                }
            }
        }

        // Sort by original index to maintain input order
        indexed_results.sort_by_key(|(index, _)| *index);

        // Extract results, converting errors to DomainResult with error info
        let results = indexed_results
            .into_iter()
            .map(|(index, result)| match result {
                Ok(domain_result) => domain_result,
                Err(e) => DomainResult {
                    domain: domains[index].clone(),
                    available: None,
                    info: None,
                    check_duration: None,
                    method_used: CheckMethod::Unknown,
                    error_message: Some(e.to_string()),
                },
            })
            .collect();

        Ok(results)
    }

    /// Check domains and return results as a stream.
    ///
    /// This method yields results as they become available, which is useful
    /// for real-time updates or when processing large numbers of domains.
    /// Results are returned in the order they complete, not input order.
    ///
    /// # Arguments
    ///
    /// * `domains` - Slice of domain names to check
    ///
    /// # Returns
    ///
    /// A stream that yields `DomainResult` items as they complete.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use domain_check_lib::DomainChecker;
    /// use futures_util::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let checker = DomainChecker::new();
    ///     let domains = vec!["example.com".to_string(), "google.com".to_string()];
    ///     
    ///     let mut stream = checker.check_domains_stream(&domains);
    ///     while let Some(result) = stream.next().await {
    ///         match result {
    ///             Ok(domain_result) => println!("✓ {}: {:?}", domain_result.domain, domain_result.available),
    ///             Err(e) => println!("✗ Error: {}", e),
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn check_domains_stream(
        &self,
        domains: &[String],
    ) -> Pin<Box<dyn Stream<Item = Result<DomainResult, DomainCheckError>> + Send + '_>> {
        let domains = domains.to_vec();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        // Create stream of futures
        let stream = futures_util::stream::iter(domains)
            .map(move |domain| {
                let semaphore = Arc::clone(&semaphore);
                let rdap_client = self.rdap_client.clone();
                let whois_client = self.whois_client.clone();
                let config = self.config.clone();

                async move {
                    // Acquire semaphore permit
                    let _permit = semaphore.acquire().await.unwrap();

                    // Check domain
                    check_single_domain_concurrent(&domain, &rdap_client, &whois_client, &config)
                        .await
                }
            })
            // Buffer unordered allows concurrent execution while maintaining the stream interface
            .buffer_unordered(self.config.concurrency);

        Box::pin(stream)
    }

    /// Read domain names from a file and check their availability.
    ///
    /// The file should contain one domain name per line. Empty lines and
    /// lines starting with '#' are ignored as comments.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file containing domain names
    ///
    /// # Returns
    ///
    /// Vector of `DomainResult` for all valid domains in the file.
    ///
    /// # Errors
    ///
    /// Returns `DomainCheckError` if:
    /// - The file cannot be read
    /// - The file contains too many domains (over limit)
    /// - No valid domains are found in the file
    pub async fn check_domains_from_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<DomainResult>, DomainCheckError> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use std::path::Path;

        // Check if file exists
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(DomainCheckError::file_error(file_path, "File not found"));
        }

        // Read domains from file
        let file = File::open(path).map_err(|e| {
            DomainCheckError::file_error(file_path, format!("Cannot open file: {}", e))
        })?;

        let reader = BufReader::new(file);
        let mut domains = Vec::new();
        let mut line_num = 0;

        for line in reader.lines() {
            line_num += 1;
            match line {
                Ok(line) => {
                    let trimmed = line.trim();

                    // Skip empty lines and comments
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }

                    // Handle inline comments
                    let domain_part = trimmed.split('#').next().unwrap_or("").trim();
                    if !domain_part.is_empty() && domain_part.len() >= 2 {
                        domains.push(domain_part.to_string());
                    }
                }
                Err(e) => {
                    return Err(DomainCheckError::file_error(
                        file_path,
                        format!("Error reading line {}: {}", line_num, e),
                    ));
                }
            }
        }

        if domains.is_empty() {
            return Err(DomainCheckError::file_error(
                file_path,
                "No valid domains found in file",
            ));
        }

        // Check domains using existing concurrent logic
        self.check_domains(&domains).await
    }

    /// Get the current configuration for this checker.
    pub fn config(&self) -> &CheckConfig {
        &self.config
    }

    /// Update the configuration for this checker.
    ///
    /// This allows modifying settings like concurrency or timeout
    /// after the checker has been created. Note that this will recreate
    /// the internal protocol clients with the new settings.
    pub fn set_config(&mut self, config: CheckConfig) {
        // Recreate clients with new configuration
        self.rdap_client = RdapClient::with_config(config.rdap_timeout, config.enable_bootstrap)
            .expect("Failed to recreate RDAP client");
        self.whois_client = WhoisClient::with_timeout(config.whois_timeout);
        self.config = config;
    }
}

impl Default for DomainChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DomainInfo;
    use std::time::Duration;

    // ── DomainChecker creation ──────────────────────────────────────────

    #[test]
    fn test_domain_checker_new() {
        let checker = DomainChecker::new();
        assert_eq!(checker.config().concurrency, 20);
        assert!(checker.config().enable_whois_fallback);
        assert!(checker.config().enable_bootstrap);
        assert!(!checker.config().detailed_info);
    }

    #[test]
    fn test_domain_checker_default() {
        let checker = DomainChecker::default();
        assert_eq!(checker.config().concurrency, 20);
    }

    #[test]
    fn test_domain_checker_with_config() {
        let config = CheckConfig::default()
            .with_concurrency(50)
            .with_timeout(Duration::from_secs(10))
            .with_detailed_info(true)
            .with_whois_fallback(false);

        let checker = DomainChecker::with_config(config);
        assert_eq!(checker.config().concurrency, 50);
        assert_eq!(checker.config().timeout, Duration::from_secs(10));
        assert!(checker.config().detailed_info);
        assert!(!checker.config().enable_whois_fallback);
    }

    // ── config() and set_config() ───────────────────────────────────────

    #[test]
    fn test_config_accessor() {
        let checker = DomainChecker::new();
        let config = checker.config();
        assert_eq!(config.concurrency, 20);
    }

    #[test]
    fn test_set_config() {
        let mut checker = DomainChecker::new();
        assert_eq!(checker.config().concurrency, 20);

        let new_config = CheckConfig::default().with_concurrency(75);
        checker.set_config(new_config);
        assert_eq!(checker.config().concurrency, 75);
    }

    // ── filter_result_info ──────────────────────────────────────────────

    #[test]
    fn test_filter_result_info_removes_when_disabled() {
        let checker = DomainChecker::new(); // detailed_info = false by default
        let result = DomainResult {
            domain: "test.com".to_string(),
            available: Some(false),
            info: Some(DomainInfo {
                registrar: Some("Test Registrar".to_string()),
                ..Default::default()
            }),
            check_duration: None,
            method_used: CheckMethod::Rdap,
            error_message: None,
        };

        let filtered = checker.filter_result_info(result);
        assert!(filtered.info.is_none());
    }

    #[test]
    fn test_filter_result_info_preserves_when_enabled() {
        let config = CheckConfig::default().with_detailed_info(true);
        let checker = DomainChecker::with_config(config);

        let result = DomainResult {
            domain: "test.com".to_string(),
            available: Some(false),
            info: Some(DomainInfo {
                registrar: Some("Test Registrar".to_string()),
                ..Default::default()
            }),
            check_duration: None,
            method_used: CheckMethod::Rdap,
            error_message: None,
        };

        let filtered = checker.filter_result_info(result);
        assert!(filtered.info.is_some());
        assert_eq!(
            filtered.info.unwrap().registrar,
            Some("Test Registrar".to_string())
        );
    }

    #[test]
    fn test_filter_result_info_no_info_noop() {
        let checker = DomainChecker::new();
        let result = DomainResult {
            domain: "test.com".to_string(),
            available: Some(true),
            info: None,
            check_duration: None,
            method_used: CheckMethod::Rdap,
            error_message: None,
        };

        let filtered = checker.filter_result_info(result);
        assert!(filtered.info.is_none());
        assert_eq!(filtered.available, Some(true));
    }

    // ── check_domains with empty list ───────────────────────────────────

    #[tokio::test]
    async fn test_check_domains_empty_list() {
        let checker = DomainChecker::new();
        let results = checker.check_domains(&[]).await.unwrap();
        assert!(results.is_empty());
    }

    // ── check_domains_from_file errors ──────────────────────────────────

    #[tokio::test]
    async fn test_check_domains_from_nonexistent_file() {
        let checker = DomainChecker::new();
        let result = checker
            .check_domains_from_file("/tmp/nonexistent_file_xyz_987.txt")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_check_domains_from_empty_file() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "# just a comment").unwrap();
        writeln!(f).unwrap();
        f.flush().unwrap();

        let checker = DomainChecker::new();
        let result = checker
            .check_domains_from_file(f.path().to_str().unwrap())
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No valid domains"));
    }

    #[tokio::test]
    async fn test_check_domains_from_file_parses_correctly() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "# Header comment").unwrap();
        writeln!(f, "example.com").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "test.org  # inline comment").unwrap();
        writeln!(f, "   ").unwrap();
        writeln!(f, "short").unwrap(); // only 5 chars but >= 2 so it's valid
        f.flush().unwrap();

        // We can't actually check domains in tests (network), but we can
        // verify the file parsing by checking that it doesn't error on
        // "no valid domains" — meaning it found at least one valid domain.
        // The actual network check will fail, but that's expected.
        let checker = DomainChecker::new();
        let result = checker
            .check_domains_from_file(f.path().to_str().unwrap())
            .await;
        // It won't error with "No valid domains" — it will either succeed or
        // fail on network. The file parsing itself worked.
        if let Err(e) = &result {
            assert!(
                !e.to_string().contains("No valid domains"),
                "File should have valid domains"
            );
        }
    }
}
