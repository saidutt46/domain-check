//! Utility functions for domain processing and validation.
//!
//! This module contains helper functions for domain name validation,
//! parsing, and other common operations used throughout the library.

use crate::error::DomainCheckError;

/// Validate a domain name format.
///
/// Checks if a domain name has valid syntax according to RFC specifications.
/// This is a basic validation - more comprehensive checks happen during lookup.
///
/// # Arguments
///
/// * `domain` - The domain name to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err(DomainCheckError)` if invalid.
pub fn validate_domain(domain: &str) -> Result<(), DomainCheckError> {
    let domain = domain.trim();

    if domain.is_empty() {
        return Err(DomainCheckError::invalid_domain(
            domain,
            "Domain name cannot be empty",
        ));
    }

    // TODO: Implement proper domain validation
    // For now, just check basic format
    if !domain.contains('.') && domain.len() < 2 {
        return Err(DomainCheckError::invalid_domain(
            domain,
            "Domain name too short",
        ));
    }

    Ok(())
}

/// Extract the base name and TLD from a domain.
///
/// Handles multi-level TLDs properly (e.g., "example.co.uk" -> ("example", "co.uk")).
///
/// # Arguments
///
/// * `domain` - The domain to parse
///
/// # Returns
///
/// A tuple of (base_name, tld) where TLD is None if no dot is found.
#[allow(dead_code)]
pub fn extract_domain_parts(domain: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = domain.split('.').collect();

    if parts.len() >= 2 {
        let base_name = parts[0].to_string();
        let tld = parts[1..].join(".");
        (base_name, Some(tld))
    } else {
        (domain.to_string(), None)
    }
}

/// Expand domain inputs based on smart detection rules.
///
/// Implements the smart expansion logic:
/// - Domains with dots are treated as FQDNs (no expansion)
/// - Domains without dots get expanded with provided TLDs
/// - Validates and filters out invalid domains
///
/// # Arguments
///
/// * `domains` - Input domain names
/// * `tlds` - TLDs to use for expansion (defaults to ["com"] if None)
///
/// # Returns
///
/// Vector of fully qualified domain names ready for checking.
pub fn expand_domain_inputs(domains: &[String], tlds: &Option<Vec<String>>) -> Vec<String> {
    let mut results = Vec::new();

    for domain in domains {
        let trimmed = domain.trim();

        // Skip empty or invalid domains
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.contains('.') {
            // Has dot = treat as FQDN (Fully Qualified Domain Name)
            // Validate basic FQDN structure
            if is_valid_fqdn(trimmed) {
                results.push(trimmed.to_string());
            }
        } else {
            // No dot = base name, expand with TLDs
            // Validate base name (minimum 2 chars, basic format)
            if is_valid_base_name(trimmed) {
                match tlds {
                    Some(tld_list) => {
                        for tld in tld_list {
                            let tld_clean = tld.trim();
                            if !tld_clean.is_empty() {
                                results.push(format!("{}.{}", trimmed, tld_clean));
                            }
                        }
                    }
                    None => {
                        // Default to .com if no TLDs specified
                        results.push(format!("{}.com", trimmed));
                    }
                }
            }
        }
    }

    results
}

/// Validate that a base domain name (without TLD) is acceptable.
pub(crate) fn is_valid_base_name(domain: &str) -> bool {
    // Minimum length check
    if domain.len() < 2 {
        return false;
    }

    // Basic character validation (alphanumeric and hyphens)
    // Cannot start or end with hyphen
    if domain.starts_with('-') || domain.ends_with('-') {
        return false;
    }

    // Only allow alphanumeric and hyphens
    domain.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// Validate that an FQDN has basic valid structure.
fn is_valid_fqdn(domain: &str) -> bool {
    // Basic checks
    if domain.len() < 4 || domain.len() > 253 {
        return false;
    }

    // Must contain at least one dot
    if !domain.contains('.') {
        return false;
    }

    // Cannot start or end with dot or hyphen
    if domain.starts_with('.')
        || domain.ends_with('.')
        || domain.starts_with('-')
        || domain.ends_with('-')
    {
        return false;
    }

    // Check each part
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    // Each part must be valid
    for part in parts {
        if part.is_empty() || part.len() > 63 {
            return false;
        }

        // Cannot start or end with hyphen
        if part.starts_with('-') || part.ends_with('-') {
            return false;
        }

        // Only alphanumeric and hyphens
        if !part.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain() {
        assert!(validate_domain("example.com").is_ok());
        assert!(validate_domain("test").is_ok());
        assert!(validate_domain("").is_err());
        assert!(validate_domain("a").is_err());
    }

    #[test]
    fn test_extract_domain_parts() {
        assert_eq!(
            extract_domain_parts("example.com"),
            ("example".to_string(), Some("com".to_string()))
        );
        assert_eq!(
            extract_domain_parts("test.co.uk"),
            ("test".to_string(), Some("co.uk".to_string()))
        );
        assert_eq!(
            extract_domain_parts("example"),
            ("example".to_string(), None)
        );
    }

    #[test]
    fn test_expand_domain_inputs() {
        let domains = vec!["example".to_string(), "test.com".to_string()];
        let tlds = Some(vec!["com".to_string(), "org".to_string()]);

        let result = expand_domain_inputs(&domains, &tlds);
        assert_eq!(
            result,
            vec![
                "example.com",
                "example.org",
                "test.com" // FQDN, no expansion
            ]
        );
    }

    #[test]
    fn test_expand_domain_inputs_with_invalid() {
        let domains = vec![
            "".to_string(),
            "a".to_string(),
            "valid".to_string(),
            "test.com".to_string(),
        ];
        let tlds = Some(vec!["com".to_string(), "org".to_string()]);

        let result = expand_domain_inputs(&domains, &tlds);
        // Should skip empty and single-char domains
        assert_eq!(result, vec!["valid.com", "valid.org", "test.com"]);
    }

    #[test]
    fn test_is_valid_base_name() {
        assert!(is_valid_base_name("example"));
        assert!(is_valid_base_name("test-domain"));
        assert!(is_valid_base_name("abc123"));

        assert!(!is_valid_base_name(""));
        assert!(!is_valid_base_name("a"));
        assert!(!is_valid_base_name("-example"));
        assert!(!is_valid_base_name("example-"));
        assert!(!is_valid_base_name("test.com")); // Contains dot
    }

    #[test]
    fn test_is_valid_fqdn() {
        assert!(is_valid_fqdn("example.com"));
        assert!(is_valid_fqdn("test.co.uk"));
        assert!(is_valid_fqdn("sub.example.com"));

        assert!(!is_valid_fqdn("example"));
        assert!(!is_valid_fqdn(".com"));
        assert!(!is_valid_fqdn("example."));
        assert!(!is_valid_fqdn("-example.com"));
        assert!(!is_valid_fqdn("example.com-"));
        assert!(!is_valid_fqdn("ex."));
    }
}
