//! Protocol implementations for domain checking.
//!
//! This module contains implementations for different protocols used
//! to check domain availability, including RDAP, WHOIS, and bootstrap registry.

/// RDAP (Registration Data Access Protocol) implementation
pub mod rdap;

/// WHOIS protocol implementation  
pub mod whois;

/// Registry mappings and bootstrap discovery
pub mod registry;

// Re-export commonly used functions and types
pub use registry::{get_rdap_endpoint, extract_tld, get_rdap_registry_map};
pub use rdap::{RdapClient, extract_domain_info};
pub use whois::{WhoisClient, is_whois_available, get_whois_version};