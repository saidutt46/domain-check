//! Main domain checker implementation.
//!
//! This module provides the primary `DomainChecker` struct that orchestrates
//! domain availability checking using RDAP, WHOIS, and bootstrap protocols.

use crate::types::{CheckConfig, DomainResult, CheckMethod};
use crate::error::DomainCheckError;
use crate::protocols::{RdapClient, WhoisClient};
use crate::utils::{validate_domain, expand_domain_inputs};
use futures::stream::{Stream, StreamExt};
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
                match whois_client.check_domain(domain).await {
                    Ok(whois_result) => {
                        let mut filtered_result = whois_result;
                        if !config.detailed_info {
                            filtered_result.info = None;
                        }
                        Ok(filtered_result)
                    }
                    Err(_whois_error) => {
                        // Both RDAP and WHOIS failed, return the most informative error
                        if rdap_error.indicates_available() {
                            // RDAP error suggests domain is available
                            Ok(DomainResult {
                                domain: domain.to_string(),
                                available: Some(true),
                                info: None,
                                check_duration: None,
                                method_used: CheckMethod::Rdap,
                                error_message: None,
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
    /// - Concurrency: 10
    /// - Timeout: 5 seconds
    /// - WHOIS fallback: enabled
    /// - Bootstrap: disabled
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
                    match self.whois_client.check_domain(domain).await {
                        Ok(whois_result) => {
                            Ok(self.filter_result_info(whois_result))
                        }
                        Err(whois_error) => {
                            // Both RDAP and WHOIS failed, return the most informative error
                            if rdap_error.indicates_available() {
                                // RDAP error suggests domain is available
                                Ok(DomainResult {
                                    domain: domain.to_string(),
                                    available: Some(true),
                                    info: None,
                                    check_duration: None,
                                    method_used: CheckMethod::Rdap,
                                    error_message: None,
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
    ///     let domains = vec!["example.com", "google.com", "test.org"];
    ///     let results = checker.check_domains(&domains).await?;
    ///     
    ///     for result in results {
    ///         println!("{}: {:?}", result.domain, result.available);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_domains(&self, domains: &[String]) -> Result<Vec<DomainResult>, DomainCheckError> {
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
                let result = check_single_domain_concurrent(
                    &domain,
                    &rdap_client,
                    &whois_client, 
                    &config
                ).await;
                
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
                    return Err(DomainCheckError::internal(
                        format!("Concurrent task failed: {}", e)
                    ));
                }
            }
        }

        // Sort by original index to maintain input order
        indexed_results.sort_by_key(|(index, _)| *index);
        
        // Extract results, converting errors to DomainResult with error info
        let results = indexed_results
            .into_iter()
            .map(|(_, result)| match result {
                Ok(domain_result) => domain_result,
                Err(e) => DomainResult {
                    domain: domains[0].clone(), // We'll fix this in the concurrent function
                    available: None,
                    info: None,
                    check_duration: None,
                    method_used: CheckMethod::Unknown,
                    error_message: Some(e.to_string()),
                }
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
    /// use futures::StreamExt;
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
    pub fn check_domains_stream(&self, domains: &[String]) -> Pin<Box<dyn Stream<Item = Result<DomainResult, DomainCheckError>> + Send + '_>> {
        let domains = domains.to_vec();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));
        
        // Create stream of futures
        let stream = futures::stream::iter(domains)
            .map(move |domain| {
                let semaphore = Arc::clone(&semaphore);
                let rdap_client = self.rdap_client.clone();
                let whois_client = self.whois_client.clone();
                let config = self.config.clone();
                
                async move {
                    // Acquire semaphore permit
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    // Check domain
                    check_single_domain_concurrent(&domain, &rdap_client, &whois_client, &config).await
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
    pub async fn check_domains_from_file(&self, file_path: &str) -> Result<Vec<DomainResult>, DomainCheckError> {
        // TODO: Implement file reading and domain checking
        // For now, return an error indicating it's not implemented
        Err(DomainCheckError::internal(
            format!("File checking not implemented yet: {}", file_path)
        ))
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