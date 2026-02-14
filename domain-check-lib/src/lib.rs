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

/// Get list of enabled features at compile time
#[allow(clippy::vec_init_then_push)] // ← Add this line
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
