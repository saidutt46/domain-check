//! Main domain checker implementation.
//!
//! This module provides the primary `DomainChecker` struct that orchestrates
//! domain availability checking using RDAP, WHOIS, and bootstrap protocols.

use crate::types::{CheckConfig, DomainResult, CheckMethod};
use crate::error::DomainCheckError;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

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
        Self {
            config: CheckConfig::default(),
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
        Self { config }
    }
    
    /// Check availability of a single domain.
    ///
    /// This is the most basic operation - check one domain and return the result.
    /// The domain should be a fully qualified domain name (e.g., "example.com").
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
        // TODO: Implement actual domain checking logic
        // For now, return a placeholder result
        Ok(DomainResult {
            domain: domain.to_string(),
            available: None,
            info: None,
            check_duration: Some(std::time::Duration::from_millis(100)),
            method_used: CheckMethod::Unknown,
            error_message: Some("Not implemented yet".to_string()),
        })
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
        // TODO: Implement concurrent domain checking
        // For now, check domains sequentially
        let mut results = Vec::new();
        
        for domain in domains {
            let result = self.check_domain(domain).await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Check domains and return results as a stream.
    ///
    /// This method yields results as they become available, which is useful
    /// for real-time updates or when processing large numbers of domains.
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
        // TODO: Implement streaming domain checking
        // For now, return a simple stream that checks domains sequentially
        let domains = domains.to_vec();
        let stream = futures::stream::iter(domains)
            .then(move |domain| async move {
                self.check_domain(&domain).await
            });
            
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
    /// after the checker has been created.
    pub fn set_config(&mut self, config: CheckConfig) {
        self.config = config;
    }
}

impl Default for DomainChecker {
    fn default() -> Self {
        Self::new()
    }
}