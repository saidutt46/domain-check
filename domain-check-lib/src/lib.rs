//! # Domain Check Library
//!
//! A fast, robust library for checking domain availability using RDAP and WHOIS protocols.
//!
//! This library provides both high-level and low-level APIs for domain availability checking,
//! with support for concurrent processing, multiple protocols, and comprehensive error handling.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use domain_check_lib::{DomainChecker, CheckConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let checker = DomainChecker::new();
//!     let result = checker.check_domain("example.com").await?;
//!     
//!     println!("Domain: {} - Available: {:?}", result.domain, result.available);
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **RDAP Protocol**: Modern registration data access protocol
//! - **WHOIS Fallback**: Automatic fallback when RDAP is unavailable
//! - **Concurrent Processing**: Efficient parallel domain checking
//! - **Bootstrap Registry**: Dynamic RDAP endpoint discovery
//! - **Configurable**: Extensive configuration options

// Re-export main public API types and functions
// This makes them available as domain_check_lib::TypeName
pub use checker::DomainChecker;
pub use config::{load_env_config, ConfigManager, FileConfig, GenerationConfig};
pub use error::DomainCheckError;
pub use protocols::registry::{
    get_all_known_tlds, get_available_presets, get_preset_tlds, get_preset_tlds_with_custom,
    get_whois_server, initialize_bootstrap,
};
pub use types::{CheckConfig, CheckMethod, DomainInfo, DomainResult, OutputMode};
pub use utils::expand_domain_inputs;

// Public modules
pub mod generate;

// Re-export generation types for convenience
pub use generate::{apply_affixes, estimate_pattern_count, expand_pattern, generate_names};
pub use types::{GenerateConfig, GenerationResult};

// Internal modules - these are not part of the public API
mod checker;
mod concurrent;
mod config;
mod error;
mod protocols;
mod types;
mod utils;

// Type alias for convenience
pub type Result<T> = std::result::Result<T, DomainCheckError>;

// Library version and metadata
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

/// Initialize the library with default settings.
///
/// This function can be called to set up global state like logging,
/// registry caches, etc. It's optional - the library will work without it.
pub fn init() {
    // Future: Initialize global caches, logging, etc.
    // For now, this is a no-op but provides future extensibility
}

/// Get library information for debugging or display purposes.
pub fn info() -> LibraryInfo {
    LibraryInfo {
        version: VERSION,
        author: AUTHOR,
        features: get_enabled_features(),
    }
}

/// Information about the library build and features
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub version: &'static str,
    pub author: &'static str,
    pub features: Vec<&'static str>,
}

/// Get list of enabled features at compile time.
// Allow: each push is behind a #[cfg(feature)], so init-then-push is the only idiomatic way.
#[allow(clippy::vec_init_then_push)]
fn get_enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    #[cfg(feature = "rdap")]
    features.push("rdap");

    #[cfg(feature = "whois")]
    features.push("whois");

    #[cfg(feature = "bootstrap")]
    features.push("bootstrap");

    #[cfg(feature = "debug")]
    features.push("debug");

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constants ──────────────────────────────────────────────────────

    #[test]
    fn test_version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_version_is_semver() {
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert!(parts.len() >= 2, "VERSION should be semver: {}", VERSION);
        for part in &parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "non-numeric semver part: {}",
                part
            );
        }
    }

    #[test]
    fn test_author_is_not_empty() {
        assert!(!AUTHOR.is_empty());
    }

    // ── init() ─────────────────────────────────────────────────────────

    #[test]
    fn test_init_does_not_panic() {
        init(); // no-op, should not panic
        init(); // idempotent
    }

    // ── info() ─────────────────────────────────────────────────────────

    #[test]
    fn test_info_version_matches_constant() {
        let info = info();
        assert_eq!(info.version, VERSION);
    }

    #[test]
    fn test_info_author_matches_constant() {
        let info = info();
        assert_eq!(info.author, AUTHOR);
    }

    #[test]
    fn test_info_has_default_features() {
        let info = info();
        // With default features enabled, rdap, whois, and bootstrap should be present
        assert!(info.features.contains(&"rdap"), "missing rdap feature");
        assert!(info.features.contains(&"whois"), "missing whois feature");
        assert!(
            info.features.contains(&"bootstrap"),
            "missing bootstrap feature"
        );
    }

    #[test]
    fn test_library_info_debug() {
        let info = info();
        let debug = format!("{:?}", info);
        assert!(debug.contains("LibraryInfo"));
        assert!(debug.contains(VERSION));
    }

    #[test]
    fn test_library_info_clone() {
        let info = info();
        let cloned = info.clone();
        assert_eq!(info.version, cloned.version);
        assert_eq!(info.author, cloned.author);
        assert_eq!(info.features, cloned.features);
    }

    // ── Result type alias ──────────────────────────────────────────────

    #[test]
    fn test_result_type_alias_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_result_type_alias_err() {
        let result: Result<i32> = Err(DomainCheckError::invalid_domain("bad", "invalid"));
        assert!(result.is_err());
    }

    // ── Public re-exports ──────────────────────────────────────────────

    #[test]
    fn test_check_config_default() {
        let config = CheckConfig::default();
        assert!(config.concurrency > 0);
    }

    #[test]
    fn test_domain_result_construction() {
        let result = DomainResult {
            domain: "example.com".to_string(),
            available: Some(true),
            info: None,
            check_duration: None,
            method_used: CheckMethod::Rdap,
            error_message: None,
        };
        assert_eq!(result.domain, "example.com");
        assert_eq!(result.available, Some(true));
    }

    #[test]
    fn test_check_method_variants() {
        let _rdap = CheckMethod::Rdap;
        let _whois = CheckMethod::Whois;
        let _unknown = CheckMethod::Unknown;
    }

    #[test]
    fn test_output_mode_variants() {
        let _streaming = OutputMode::Streaming;
        let _collected = OutputMode::Collected;
        let _auto = OutputMode::Auto;
    }

    #[test]
    fn test_domain_checker_new() {
        let checker = DomainChecker::new();
        let config = checker.config();
        assert!(config.concurrency > 0);
    }
}
