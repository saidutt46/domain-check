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

// Re-export core types that external users might need
pub use rdap::RdapClient;
pub use whois::WhoisClient;
