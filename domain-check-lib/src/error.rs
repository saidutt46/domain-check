//! Error handling for domain checking operations.
//!
//! This module defines a comprehensive error type that covers all the different
//! ways domain checking can fail, from network issues to invalid input.

use std::fmt;

/// Main error type for domain checking operations.
///
/// This enum covers all possible failure modes in the domain checking process,
/// providing detailed context for debugging and user-friendly error messages.
#[derive(Debug, Clone)]
pub enum DomainCheckError {
    /// Invalid domain name format
    InvalidDomain {
        domain: String,
        reason: String,
    },
    
    /// Network-related errors (connection, timeout, etc.)
    NetworkError {
        message: String,
        source: Option<String>,
    },
    
    /// RDAP protocol specific errors
    RdapError {
        domain: String,
        message: String,
        status_code: Option<u16>,
    },
    
    /// WHOIS protocol specific errors
    WhoisError {
        domain: String,
        message: String,
    },
    
    /// Bootstrap registry lookup failures
    BootstrapError {
        tld: String,
        message: String,
    },
    
    /// JSON parsing errors for RDAP responses
    ParseError {
        message: String,
        content: Option<String>,
    },
    
    /// Configuration errors (invalid settings, etc.)
    ConfigError {
        message: String,
    },
    
    /// File I/O errors when reading domain lists
    FileError {
        path: String,
        message: String,
    },
    
    /// Timeout errors when operations take too long
    Timeout {
        operation: String,
        duration: std::time::Duration,
    },
    
    /// Rate limiting errors when servers reject requests
    RateLimited {
        service: String,
        message: String,
        retry_after: Option<std::time::Duration>,
    },
    
    /// Generic internal errors that don't fit other categories
    Internal {
        message: String,
    },
}

impl DomainCheckError {
    /// Create a new invalid domain error.
    pub fn invalid_domain<D: Into<String>, R: Into<String>>(domain: D, reason: R) -> Self {
        Self::InvalidDomain {
            domain: domain.into(),
            reason: reason.into(),
        }
    }
    
    /// Create a new network error.
    pub fn network<M: Into<String>>(message: M) -> Self {
        Self::NetworkError {
            message: message.into(),
            source: None,
        }
    }
    
    /// Create a new network error with source information.
    pub fn network_with_source<M: Into<String>, S: Into<String>>(message: M, source: S) -> Self {
        Self::NetworkError {
            message: message.into(),
            source: Some(source.into()),
        }
    }
    
    /// Create a new RDAP error.
    pub fn rdap<D: Into<String>, M: Into<String>>(domain: D, message: M) -> Self {
        Self::RdapError {
            domain: domain.into(),
            message: message.into(),
            status_code: None,
        }
    }
    
    /// Create a new RDAP error with HTTP status code.
    pub fn rdap_with_status<D: Into<String>, M: Into<String>>(domain: D, message: M, status_code: u16) -> Self {
        Self::RdapError {
            domain: domain.into(),
            message: message.into(),
            status_code: Some(status_code),
        }
    }
    
    /// Create a new WHOIS error.
    pub fn whois<D: Into<String>, M: Into<String>>(domain: D, message: M) -> Self {
        Self::WhoisError {
            domain: domain.into(),
            message: message.into(),
        }
    }
    
    /// Create a new bootstrap error.
    pub fn bootstrap<T: Into<String>, M: Into<String>>(tld: T, message: M) -> Self {
        Self::BootstrapError {
            tld: tld.into(),
            message: message.into(),
        }
    }
    
    /// Create a new timeout error.
    pub fn timeout<O: Into<String>>(operation: O, duration: std::time::Duration) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration,
        }
    }
    
    /// Create a new internal error.
    pub fn internal<M: Into<String>>(message: M) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
    
    /// Create a new file error.
    pub fn file_error<P: Into<String>, M: Into<String>>(path: P, message: M) -> Self {
        Self::FileError {
            path: path.into(),
            message: message.into(),
        }
    }
    
    /// Check if this error indicates the domain is definitely available.
    /// 
    /// Some error conditions (like NXDOMAIN) actually indicate availability.
    pub fn indicates_available(&self) -> bool {
        match self {
            Self::RdapError { status_code: Some(404), .. } => true,
            Self::WhoisError { message, .. } => {
                let msg = message.to_lowercase();
                msg.contains("not found") || 
                msg.contains("no match") || 
                msg.contains("no data found") ||
                msg.contains("domain available")
            },
            _ => false,
        }
    }
    
    /// Check if this error suggests the operation should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            Self::NetworkError { .. } |
            Self::Timeout { .. } |
            Self::RateLimited { .. } |
            Self::RdapError { status_code: Some(500..=599), .. }
        )
    }
}

impl fmt::Display for DomainCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomain { domain, reason } => {
                write!(f, "Invalid domain '{}': {}", domain, reason)
            }
            Self::NetworkError { message, source } => {
                if let Some(source) = source {
                    write!(f, "Network error: {} (source: {})", message, source)
                } else {
                    write!(f, "Network error: {}", message)
                }
            }
            Self::RdapError { domain, message, status_code } => {
                if let Some(code) = status_code {
                    write!(f, "RDAP error for '{}' (HTTP {}): {}", domain, code, message)
                } else {
                    write!(f, "RDAP error for '{}': {}", domain, message)
                }
            }
            Self::WhoisError { domain, message } => {
                write!(f, "WHOIS error for '{}': {}", domain, message)
            }
            Self::BootstrapError { tld, message } => {
                write!(f, "Bootstrap error for TLD '{}': {}", tld, message)
            }
            Self::ParseError { message, content: _ } => {
                write!(f, "Parse error: {}", message)
            }
            Self::ConfigError { message } => {
                write!(f, "Configuration error: {}", message)
            }
            Self::FileError { path, message } => {
                write!(f, "File error at '{}': {}", path, message)
            }
            Self::Timeout { operation, duration } => {
                write!(f, "Timeout after {:?} during: {}", duration, operation)
            }
            Self::RateLimited { service, message, retry_after } => {
                if let Some(retry) = retry_after {
                    write!(f, "Rate limited by {} (retry after {:?}): {}", service, retry, message)
                } else {
                    write!(f, "Rate limited by {}: {}", service, message)
                }
            }
            Self::Internal { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for DomainCheckError {}

// Implement From conversions for common error types
impl From<reqwest::Error> for DomainCheckError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::timeout("HTTP request", std::time::Duration::from_secs(30))
        } else if err.is_connect() {
            Self::network_with_source("Connection failed", err.to_string())
        } else {
            Self::network_with_source("HTTP request failed", err.to_string())
        }
    }
}

impl From<serde_json::Error> for DomainCheckError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError {
            message: format!("JSON parsing failed: {}", err),
            content: None,
        }
    }
}

impl From<std::io::Error> for DomainCheckError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal {
            message: format!("I/O error: {}", err),
        }
    }
}

impl From<regex::Error> for DomainCheckError {
    fn from(err: regex::Error) -> Self {
        Self::Internal {
            message: format!("Regex error: {}", err),
        }
    }
}