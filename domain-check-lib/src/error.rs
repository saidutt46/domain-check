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

    /// Invalid pattern syntax in domain generation
    InvalidPattern { pattern: String, reason: String },

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

    /// Create a new invalid pattern error.
    pub fn invalid_pattern<P: Into<String>, R: Into<String>>(pattern: P, reason: R) -> Self {
        Self::InvalidPattern {
            pattern: pattern.into(),
            reason: reason.into(),
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
        // InvalidPattern is not retryable — it's a user input error
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
                    let _ = source; // source captured for Debug/error chain, not user-facing
                    write!(f, "🌐 Network error: {}\n   💡 Please check your internet connection", message)
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
            Self::InvalidPattern { pattern, reason } => {
                write!(f, "⚙️ Invalid pattern '{}': {}\n   💡 Supported: \\w (letters+hyphen), \\d (digits), ? (alphanumeric), literal characters", pattern, reason)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constructor tests ───────────────────────────────────────────────

    #[test]
    fn test_invalid_domain_constructor() {
        let err = DomainCheckError::invalid_domain("bad!", "contains special characters");
        match err {
            DomainCheckError::InvalidDomain { domain, reason } => {
                assert_eq!(domain, "bad!");
                assert_eq!(reason, "contains special characters");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_network_constructor() {
        let err = DomainCheckError::network("connection refused");
        match err {
            DomainCheckError::NetworkError { message, source } => {
                assert_eq!(message, "connection refused");
                assert!(source.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_network_with_source_constructor() {
        let err = DomainCheckError::network_with_source("failed", "dns lookup error");
        match err {
            DomainCheckError::NetworkError { message, source } => {
                assert_eq!(message, "failed");
                assert_eq!(source, Some("dns lookup error".to_string()));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_rdap_constructor() {
        let err = DomainCheckError::rdap("test.com", "lookup failed");
        match err {
            DomainCheckError::RdapError {
                domain,
                message,
                status_code,
            } => {
                assert_eq!(domain, "test.com");
                assert_eq!(message, "lookup failed");
                assert!(status_code.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_rdap_with_status_constructor() {
        let err = DomainCheckError::rdap_with_status("test.com", "not found", 404);
        match err {
            DomainCheckError::RdapError {
                domain,
                message,
                status_code,
            } => {
                assert_eq!(domain, "test.com");
                assert_eq!(message, "not found");
                assert_eq!(status_code, Some(404));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_whois_constructor() {
        let err = DomainCheckError::whois("test.com", "server unreachable");
        match err {
            DomainCheckError::WhoisError { domain, message } => {
                assert_eq!(domain, "test.com");
                assert_eq!(message, "server unreachable");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_bootstrap_constructor() {
        let err = DomainCheckError::bootstrap("xyz", "no endpoint found");
        match err {
            DomainCheckError::BootstrapError { tld, message } => {
                assert_eq!(tld, "xyz");
                assert_eq!(message, "no endpoint found");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_timeout_constructor() {
        let err = DomainCheckError::timeout("RDAP lookup", std::time::Duration::from_secs(10));
        match err {
            DomainCheckError::Timeout {
                operation,
                duration,
            } => {
                assert_eq!(operation, "RDAP lookup");
                assert_eq!(duration, std::time::Duration::from_secs(10));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_internal_constructor() {
        let err = DomainCheckError::internal("unexpected state");
        match err {
            DomainCheckError::Internal { message } => {
                assert_eq!(message, "unexpected state");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_file_error_constructor() {
        let err = DomainCheckError::file_error("/tmp/domains.txt", "permission denied");
        match err {
            DomainCheckError::FileError { path, message } => {
                assert_eq!(path, "/tmp/domains.txt");
                assert_eq!(message, "permission denied");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_invalid_pattern_constructor() {
        let err = DomainCheckError::invalid_pattern("test\\x", "unknown escape");
        match err {
            DomainCheckError::InvalidPattern { pattern, reason } => {
                assert_eq!(pattern, "test\\x");
                assert_eq!(reason, "unknown escape");
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── indicates_available ─────────────────────────────────────────────

    #[test]
    fn test_rdap_404_indicates_available() {
        let err = DomainCheckError::rdap_with_status("test.com", "not found", 404);
        assert!(err.indicates_available());
    }

    #[test]
    fn test_rdap_200_not_available() {
        let err = DomainCheckError::rdap_with_status("test.com", "ok", 200);
        assert!(!err.indicates_available());
    }

    #[test]
    fn test_rdap_no_status_not_available() {
        let err = DomainCheckError::rdap("test.com", "generic error");
        assert!(!err.indicates_available());
    }

    #[test]
    fn test_whois_not_found_indicates_available() {
        let err = DomainCheckError::whois("test.com", "No match for domain NOT FOUND");
        assert!(err.indicates_available());
    }

    #[test]
    fn test_whois_no_data_found_indicates_available() {
        let err = DomainCheckError::whois("test.com", "No Data Found");
        assert!(err.indicates_available());
    }

    #[test]
    fn test_whois_domain_available_indicates_available() {
        let err = DomainCheckError::whois("test.com", "Domain Available for registration");
        assert!(err.indicates_available());
    }

    #[test]
    fn test_whois_rate_limit_not_available() {
        let err = DomainCheckError::whois("test.com", "rate limited");
        assert!(!err.indicates_available());
    }

    #[test]
    fn test_network_error_not_available() {
        let err = DomainCheckError::network("connection refused");
        assert!(!err.indicates_available());
    }

    #[test]
    fn test_timeout_not_available() {
        let err = DomainCheckError::timeout("test", std::time::Duration::from_secs(5));
        assert!(!err.indicates_available());
    }

    #[test]
    fn test_invalid_pattern_not_available() {
        let err = DomainCheckError::invalid_pattern("bad", "reason");
        assert!(!err.indicates_available());
    }

    // ── is_retryable ────────────────────────────────────────────────────

    #[test]
    fn test_network_error_is_retryable() {
        let err = DomainCheckError::network("connection refused");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_network_with_source_is_retryable() {
        let err = DomainCheckError::network_with_source("failed", "dns");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_timeout_is_retryable() {
        let err = DomainCheckError::timeout("test", std::time::Duration::from_secs(5));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_rate_limited_is_retryable() {
        let err = DomainCheckError::RateLimited {
            service: "RDAP".to_string(),
            message: "too many requests".to_string(),
            retry_after: Some(std::time::Duration::from_secs(30)),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_rdap_500_is_retryable() {
        let err = DomainCheckError::rdap_with_status("test.com", "server error", 500);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_rdap_502_is_retryable() {
        let err = DomainCheckError::rdap_with_status("test.com", "bad gateway", 502);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_rdap_599_is_retryable() {
        let err = DomainCheckError::rdap_with_status("test.com", "error", 599);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_rdap_403_not_retryable() {
        let err = DomainCheckError::rdap_with_status("test.com", "forbidden", 403);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_rdap_404_not_retryable() {
        let err = DomainCheckError::rdap_with_status("test.com", "not found", 404);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_config_error_not_retryable() {
        let err = DomainCheckError::ConfigError {
            message: "bad config".to_string(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_invalid_domain_not_retryable() {
        let err = DomainCheckError::invalid_domain("bad", "too short");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_invalid_pattern_not_retryable() {
        let err = DomainCheckError::invalid_pattern("bad", "reason");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_file_error_not_retryable() {
        let err = DomainCheckError::file_error("/tmp/x", "not found");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_bootstrap_error_not_retryable() {
        let err = DomainCheckError::bootstrap("xyz", "no endpoint");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_internal_error_not_retryable() {
        let err = DomainCheckError::internal("unexpected");
        assert!(!err.is_retryable());
    }

    // ── Display for every variant ───────────────────────────────────────

    #[test]
    fn test_display_invalid_domain() {
        let err = DomainCheckError::invalid_domain("x", "too short");
        let msg = format!("{}", err);
        assert!(msg.contains("x"));
        assert!(msg.contains("too short"));
    }

    #[test]
    fn test_display_network_connection_error() {
        let err = DomainCheckError::network("connection refused");
        let msg = format!("{}", err);
        assert!(msg.contains("connect"));
    }

    #[test]
    fn test_display_network_timeout_error() {
        let err = DomainCheckError::network("request timeout");
        let msg = format!("{}", err);
        assert!(msg.contains("timed out") || msg.contains("timeout"));
    }

    #[test]
    fn test_display_network_generic() {
        let err = DomainCheckError::network("something broke");
        let msg = format!("{}", err);
        assert!(msg.contains("something broke"));
    }

    #[test]
    fn test_display_rdap_404() {
        let err = DomainCheckError::rdap_with_status("avail.com", "not found", 404);
        let msg = format!("{}", err);
        assert!(msg.contains("avail.com"));
        assert!(msg.contains("available"));
    }

    #[test]
    fn test_display_rdap_429() {
        let err = DomainCheckError::rdap_with_status("test.com", "rate limited", 429);
        let msg = format!("{}", err);
        assert!(msg.contains("rate limiting"));
    }

    #[test]
    fn test_display_rdap_5xx() {
        let err = DomainCheckError::rdap_with_status("test.com", "error", 503);
        let msg = format!("{}", err);
        assert!(msg.contains("temporarily unavailable"));
    }

    #[test]
    fn test_display_rdap_other_status() {
        let err = DomainCheckError::rdap_with_status("test.com", "weird", 418);
        let msg = format!("{}", err);
        assert!(msg.contains("418"));
    }

    #[test]
    fn test_display_rdap_no_status() {
        let err = DomainCheckError::rdap("test.com", "lookup failed");
        let msg = format!("{}", err);
        assert!(msg.contains("lookup failed"));
    }

    #[test]
    fn test_display_whois_not_found() {
        let err = DomainCheckError::whois("test.com", "not found");
        let msg = format!("{}", err);
        assert!(msg.contains("available"));
    }

    #[test]
    fn test_display_whois_rate_limit() {
        let err = DomainCheckError::whois("test.com", "too many requests");
        let msg = format!("{}", err);
        assert!(msg.contains("rate limiting"));
    }

    #[test]
    fn test_display_whois_generic() {
        let err = DomainCheckError::whois("test.com", "server error");
        let msg = format!("{}", err);
        assert!(msg.contains("WHOIS lookup failed"));
    }

    #[test]
    fn test_display_bootstrap_error() {
        let err = DomainCheckError::bootstrap("xyz", "no endpoint");
        let msg = format!("{}", err);
        assert!(msg.contains(".xyz"));
    }

    #[test]
    fn test_display_parse_error() {
        let err = DomainCheckError::ParseError {
            message: "bad json".to_string(),
            content: None,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("server response"));
    }

    #[test]
    fn test_display_config_error() {
        let err = DomainCheckError::ConfigError {
            message: "invalid value".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("invalid value"));
    }

    #[test]
    fn test_display_file_not_found() {
        let err = DomainCheckError::file_error("/tmp/x.txt", "no such file");
        let msg = format!("{}", err);
        assert!(msg.contains("not found") || msg.contains("no such file"));
    }

    #[test]
    fn test_display_file_permission() {
        let err = DomainCheckError::file_error("/tmp/x.txt", "permission denied");
        let msg = format!("{}", err);
        assert!(msg.contains("Permission denied") || msg.contains("permission"));
    }

    #[test]
    fn test_display_file_no_valid_domains() {
        let err = DomainCheckError::file_error("/tmp/x.txt", "no valid domains found");
        let msg = format!("{}", err);
        assert!(msg.contains("No valid domains"));
    }

    #[test]
    fn test_display_file_generic() {
        let err = DomainCheckError::file_error("/tmp/x.txt", "corrupt data");
        let msg = format!("{}", err);
        assert!(msg.contains("corrupt data"));
    }

    #[test]
    fn test_display_timeout() {
        let err = DomainCheckError::timeout("RDAP", std::time::Duration::from_secs(5));
        let msg = format!("{}", err);
        assert!(msg.contains("timed out"));
        assert!(msg.contains("RDAP"));
    }

    #[test]
    fn test_display_rate_limited_with_retry() {
        let err = DomainCheckError::RateLimited {
            service: "RDAP".to_string(),
            message: "slow down".to_string(),
            retry_after: Some(std::time::Duration::from_secs(30)),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("RDAP"));
        assert!(msg.contains("30"));
    }

    #[test]
    fn test_display_rate_limited_without_retry() {
        let err = DomainCheckError::RateLimited {
            service: "WHOIS".to_string(),
            message: "slow down".to_string(),
            retry_after: None,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("WHOIS"));
        assert!(msg.contains("wait a moment"));
    }

    #[test]
    fn test_display_invalid_pattern() {
        let err = DomainCheckError::invalid_pattern("test\\x", "unknown escape sequence '\\x'");
        let msg = format!("{}", err);
        assert!(msg.contains("test\\x"));
        assert!(msg.contains("\\w")); // hint
        assert!(msg.contains("\\d")); // hint
    }

    #[test]
    fn test_display_internal() {
        let err = DomainCheckError::internal("lock poisoned");
        let msg = format!("{}", err);
        assert!(msg.contains("lock poisoned"));
        assert!(msg.contains("Internal error") || msg.contains("unexpected"));
    }

    // ── From conversions ────────────────────────────────────────────────

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err: DomainCheckError = json_err.into();
        match err {
            DomainCheckError::ParseError { message, content } => {
                assert!(message.contains("JSON parsing failed"));
                assert!(content.is_none());
            }
            _ => panic!("expected ParseError, got {:?}", err),
        }
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: DomainCheckError = io_err.into();
        match err {
            DomainCheckError::Internal { message } => {
                assert!(message.contains("I/O error"));
                assert!(message.contains("file missing"));
            }
            _ => panic!("expected Internal, got {:?}", err),
        }
    }

    // ── std::error::Error trait ─────────────────────────────────────────

    #[test]
    fn test_error_trait_implemented() {
        let err = DomainCheckError::network("test");
        // Verify it can be used as a trait object
        let _: &dyn std::error::Error = &err;
    }
}
