//! Core data types for domain availability checking.
//!
//! This module defines all the main data structures used throughout the library,
//! including domain results, configuration options, and output formatting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Result of a domain availability check.
///
/// Contains all information about a domain's availability status,
/// registration details, and metadata about the check itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainResult {
    /// The domain name that was checked (e.g., "example.com")
    pub domain: String,

    /// Whether the domain is available for registration.
    /// - `Some(true)`: Domain is available
    /// - `Some(false)`: Domain is taken/registered  
    /// - `None`: Status could not be determined
    pub available: Option<bool>,

    /// Detailed registration information (only available for taken domains)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<DomainInfo>,

    /// How long the domain check took to complete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_duration: Option<Duration>,

    /// Which method was used to check the domain
    pub method_used: CheckMethod,

    /// Any error message if the check failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Detailed information about a registered domain.
///
/// This information is typically extracted from RDAP responses
/// and provides insights into the domain's registration details.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainInfo {
    /// The registrar that manages this domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registrar: Option<String>,

    /// When the domain was first registered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,

    /// When the domain registration expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    /// Domain status codes (e.g., "clientTransferProhibited")
    pub status: Vec<String>,

    /// Last update date of the domain record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_date: Option<String>,

    /// Nameservers associated with the domain
    pub nameservers: Vec<String>,
}

/// Configuration options for domain checking operations.
///
/// This struct allows fine-tuning of the domain checking behavior,
/// including performance, timeout, and protocol preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Maximum number of concurrent domain checks
    /// Default: 10, Range: 1-100
    pub concurrency: usize,

    /// Timeout for each individual domain check
    /// Default: 5 seconds
    #[serde(skip)] // Don't serialize Duration directly
    pub timeout: Duration,

    /// Whether to automatically fall back to WHOIS when RDAP fails
    /// Default: true
    pub enable_whois_fallback: bool,

    /// Whether to use IANA bootstrap registry for unknown TLDs
    /// Default: false (uses built-in registry only)
    pub enable_bootstrap: bool,

    /// Whether to extract detailed domain information for taken domains
    /// Default: false (just availability status)
    pub detailed_info: bool,

    /// List of TLDs to check for base domain names
    /// If None, defaults to ["com"]
    pub tlds: Option<Vec<String>>,

    /// Custom timeout for RDAP requests (separate from overall timeout)
    /// Default: 3 seconds
    #[serde(skip)] // Don't serialize Duration directly
    pub rdap_timeout: Duration,

    /// Custom timeout for WHOIS requests
    /// Default: 5 seconds  
    #[serde(skip)] // Don't serialize Duration directly
    pub whois_timeout: Duration,

    /// Custom user-defined TLD presets from config files
    /// Default: empty
    #[serde(skip)] // Handled separately in config merging
    pub custom_presets: HashMap<String, Vec<String>>,
}

/// Method used to check domain availability.
///
/// This helps users understand which protocol was used
/// and can be useful for debugging or performance analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckMethod {
    /// Domain checked via RDAP protocol
    #[serde(rename = "rdap")]
    Rdap,

    /// Domain checked via WHOIS protocol
    #[serde(rename = "whois")]
    Whois,

    /// RDAP endpoint discovered via IANA bootstrap registry
    #[serde(rename = "bootstrap")]
    Bootstrap,

    /// Check failed or method unknown
    #[serde(rename = "unknown")]
    Unknown,
}

/// Output mode for displaying results.
///
/// This controls how and when results are presented to the user,
/// affecting both performance perception and data formatting.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputMode {
    /// Stream results as they become available (good for interactive use)
    Streaming,

    /// Collect all results before displaying (good for formatting/sorting)
    Collected,

    /// Automatically choose based on context (terminal vs pipe, etc.)
    Auto,
}

impl Default for CheckConfig {
    /// Create a sensible default configuration.
    ///
    /// These defaults are chosen to work well for most use cases
    /// while being conservative about resource usage.
    fn default() -> Self {
        Self {
            concurrency: 10,
            timeout: Duration::from_secs(5),
            enable_whois_fallback: true,
            enable_bootstrap: false,
            detailed_info: false,
            tlds: None, // Will default to ["com"] when needed
            rdap_timeout: Duration::from_secs(3),
            whois_timeout: Duration::from_secs(5),
            custom_presets: HashMap::new(),
        }
    }
}

impl CheckConfig {
    /// Create a new configuration with custom concurrency.
    ///
    /// Automatically caps concurrency at 100 to prevent resource exhaustion.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.clamp(1, 100);
        self
    }

    /// Set custom timeout for domain checks.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable WHOIS fallback.
    pub fn with_whois_fallback(mut self, enabled: bool) -> Self {
        self.enable_whois_fallback = enabled;
        self
    }

    /// Enable or disable IANA bootstrap registry.
    pub fn with_bootstrap(mut self, enabled: bool) -> Self {
        self.enable_bootstrap = enabled;
        self
    }

    /// Enable detailed domain information extraction.
    pub fn with_detailed_info(mut self, enabled: bool) -> Self {
        self.detailed_info = enabled;
        self
    }

    /// Set TLDs to check for base domain names.
    pub fn with_tlds(mut self, tlds: Vec<String>) -> Self {
        self.tlds = Some(tlds);
        self
    }
}

impl std::fmt::Display for CheckMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckMethod::Rdap => write!(f, "RDAP"),
            CheckMethod::Whois => write!(f, "WHOIS"),
            CheckMethod::Bootstrap => write!(f, "Bootstrap"),
            CheckMethod::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMode::Streaming => write!(f, "Streaming"),
            OutputMode::Collected => write!(f, "Collected"),
            OutputMode::Auto => write!(f, "Auto"),
        }
    }
}
