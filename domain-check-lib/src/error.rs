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
    InvalidDomain { domain: String, reason: String },

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
    WhoisError { domain: String, message: String },

    /// Bootstrap registry lookup failures
    BootstrapError { tld: String, message: String },

    /// JSON parsing errors for RDAP responses
    ParseError {
        message: String,
        content: Option<String>,
    },

    /// Configuration errors (invalid settings, etc.)
    ConfigError { message: String },

    /// File I/O errors when reading domain lists
    FileError { path: String, message: String },

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
    Internal { message: String },
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
    pub fn rdap_with_status<D: Into<String>, M: Into<String>>(
        domain: D,
        message: M,
        status_code: u16,
    ) -> Self {
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
            Self::RdapError {
                status_code: Some(404),
                ..
            } => true,
            Self::WhoisError { message, .. } => {
                let msg = message.to_lowercase();
                msg.contains("not found")
                    || msg.contains("no match")
                    || msg.contains("no data found")
                    || msg.contains("domain available")
            }
            _ => false,
        }
    }

    /// Check if this error suggests the operation should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NetworkError { .. }
                | Self::Timeout { .. }
                | Self::RateLimited { .. }
                | Self::RdapError {
                    status_code: Some(500..=599),
                    ..
                }
        )
    }
}

impl fmt::Display for DomainCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomain { domain, reason } => {
                write!(f, "❌ '{}' is not a valid domain name: {}\n   💡 Try something like 'example.com' or use a different domain", domain, reason)
            }
            Self::NetworkError { message, source } => {
                if message.to_lowercase().contains("connection") || message.to_lowercase().contains("connect") {
                    write!(f, "🌐 Cannot connect to the internet\n   💡 Please check your network connection and try again")
                } else if message.to_lowercase().contains("timeout") {
                    write!(f, "⏱️ Request timed out\n   💡 Your internet connection may be slow. Try again or check fewer domains at once")
                } else {
                    match source {
                        Some(_) => write!(f, "🌐 Network error: {}\n   💡 Please check your internet connection", message),
                        None => write!(f, "🌐 Network error: {}\n   💡 Please check your internet connection", message),
                    }
                }
            }
            Self::RdapError { domain, message, status_code } => {
                match status_code {
                    Some(404) => write!(f, "✅ {}: Domain appears to be available", domain),
                    Some(429) => write!(f, "⏳ {}: Registry is rate limiting requests\n   💡 Please wait a moment and try again", domain),
                    Some(500..=599) => write!(f, "⚠️ {}: Registry server is temporarily unavailable\n   💡 Trying backup method...", domain),
                    Some(code) => write!(f, "⚠️ {}: Registry returned error (HTTP {})\n   💡 This domain registry may be temporarily unavailable", domain, code),
                    None => write!(f, "⚠️ {}: {}\n   💡 Trying alternative checking method...", domain, message),
                }
            }
            Self::WhoisError { domain, message } => {
                if message.to_lowercase().contains("not found") || message.to_lowercase().contains("no match") {
                    write!(f, "✅ {}: Domain appears to be available", domain)
                } else if message.to_lowercase().contains("rate limit") || message.to_lowercase().contains("too many") {
                    write!(f, "⏳ {}: WHOIS server is rate limiting requests\n   💡 Please wait a moment and try again", domain)
                } else if message.to_lowercase().contains("whois") && message.to_lowercase().contains("not found") {
                    write!(f, "⚠️ {}: WHOIS command not found on this system\n   💡 Please install whois or use online domain checkers", domain)
                } else {
                    write!(f, "⚠️ {}: WHOIS lookup failed\n   💡 This may indicate the domain is available or the server is busy", domain)
                }
            }
            Self::BootstrapError { tld, message: _ } => {
                write!(f, "❓ Unknown domain extension '.{}'\n   💡 This TLD may not support automated checking. Try manually checking with a registrar", tld)
            }
            Self::ParseError { message: _, content: _ } => {
                write!(f, "⚠️ Unable to understand server response\n   💡 The domain registry may be experiencing issues. Please try again later")
            }
            Self::ConfigError { message } => {
                write!(f, "⚙️ Configuration error: {}\n   💡 Please check your command line arguments or configuration file values", message)
            }
            Self::FileError { path, message } => {
                if message.to_lowercase().contains("not found") || message.to_lowercase().contains("no such file") {
                    write!(f, "📁 File not found: {}\n   💡 Please check the file path and make sure the file exists", path)
                } else if message.to_lowercase().contains("permission") {
                    write!(f, "🔒 Permission denied: {}\n   💡 Please check file permissions or try running with appropriate access", path)
                } else if message.to_lowercase().contains("no valid domains") {
                    write!(f, "📄 No valid domains found in: {}\n   💡 Make sure the file contains domain names (one per line) and check the format", path)
                } else {
                    write!(f, "📁 File error ({}): {}\n   💡 Please check the file and try again", path, message)
                }
            }
            Self::Timeout { operation, duration } => {
                write!(f, "⏱️ Operation timed out after {:?}: {}\n   💡 Try reducing the number of domains or check your internet connection", duration, operation)
            }
            Self::RateLimited { service, message, retry_after } => {
                match retry_after {
                    Some(retry) => write!(f, "⏳ Rate limited by {}: {}\n   💡 Please wait {:?} and try again", service, message, retry),
                    None => write!(f, "⏳ Rate limited by {}: {}\n   💡 Please wait a moment and try again", service, message),
                }
            }
            Self::Internal { message } => {
                write!(f, "🔧 Internal error: {}\n   💡 This is unexpected. Please try again or report this issue", message)
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
