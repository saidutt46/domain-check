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

    if !domain.contains('.') && domain.len() < 2 {
        return Err(DomainCheckError::invalid_domain(
            domain,
            "Domain name too short",
        ));
    }

    Ok(())
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

    // ── validate_domain ─────────────────────────────────────────────────

    #[test]
    fn test_validate_domain_valid() {
        assert!(validate_domain("example.com").is_ok());
        assert!(validate_domain("test").is_ok()); // >= 2 chars, no dot required
        assert!(validate_domain("ab").is_ok());
        assert!(validate_domain("sub.example.com").is_ok());
    }

    #[test]
    fn test_validate_domain_empty() {
        let err = validate_domain("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_validate_domain_too_short() {
        let err = validate_domain("a").unwrap_err();
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn test_validate_domain_whitespace_trimmed() {
        assert!(validate_domain("  example.com  ").is_ok());
    }

    #[test]
    fn test_validate_domain_only_whitespace() {
        assert!(validate_domain("   ").is_err());
    }

    // ── expand_domain_inputs ────────────────────────────────────────────

    #[test]
    fn test_expand_with_tlds() {
        let domains = vec!["example".to_string(), "test.com".to_string()];
        let tlds = Some(vec!["com".to_string(), "org".to_string()]);

        let result = expand_domain_inputs(&domains, &tlds);
        assert_eq!(result, vec!["example.com", "example.org", "test.com"]);
    }

    #[test]
    fn test_expand_default_tld() {
        let domains = vec!["example".to_string()];
        let result = expand_domain_inputs(&domains, &None);
        assert_eq!(result, vec!["example.com"]);
    }

    #[test]
    fn test_expand_fqdn_not_expanded() {
        let domains = vec!["already.io".to_string()];
        let tlds = Some(vec!["com".to_string(), "org".to_string()]);
        let result = expand_domain_inputs(&domains, &tlds);
        assert_eq!(result, vec!["already.io"]);
    }

    #[test]
    fn test_expand_skips_empty() {
        let domains = vec!["".to_string(), "valid".to_string()];
        let result = expand_domain_inputs(&domains, &None);
        assert_eq!(result, vec!["valid.com"]);
    }

    #[test]
    fn test_expand_skips_short_base_names() {
        let domains = vec!["a".to_string(), "valid".to_string()];
        let result = expand_domain_inputs(&domains, &None);
        assert_eq!(result, vec!["valid.com"]);
    }

    #[test]
    fn test_expand_skips_invalid_fqdn() {
        let domains = vec![".com".to_string(), "example.com".to_string()];
        let result = expand_domain_inputs(&domains, &None);
        assert_eq!(result, vec!["example.com"]);
    }

    #[test]
    fn test_expand_empty_input() {
        let result = expand_domain_inputs(&[], &None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_expand_empty_tld_entries_filtered() {
        let domains = vec!["example".to_string()];
        let tlds = Some(vec!["com".to_string(), "".to_string(), "org".to_string()]);
        let result = expand_domain_inputs(&domains, &tlds);
        assert_eq!(result, vec!["example.com", "example.org"]);
    }

    #[test]
    fn test_expand_mixed_fqdn_and_base() {
        let domains = vec![
            "base1".to_string(),
            "already.io".to_string(),
            "base2".to_string(),
        ];
        let tlds = Some(vec!["com".to_string()]);
        let result = expand_domain_inputs(&domains, &tlds);
        assert_eq!(result, vec!["base1.com", "already.io", "base2.com"]);
    }

    // ── is_valid_base_name ──────────────────────────────────────────────

    #[test]
    fn test_valid_base_names() {
        assert!(is_valid_base_name("example"));
        assert!(is_valid_base_name("test-domain"));
        assert!(is_valid_base_name("abc123"));
        assert!(is_valid_base_name("ab")); // minimum valid
        assert!(is_valid_base_name("a1"));
    }

    #[test]
    fn test_invalid_base_names() {
        assert!(!is_valid_base_name(""));
        assert!(!is_valid_base_name("a")); // too short
        assert!(!is_valid_base_name("-example")); // starts with hyphen
        assert!(!is_valid_base_name("example-")); // ends with hyphen
        assert!(!is_valid_base_name("test.com")); // contains dot
        assert!(!is_valid_base_name("test domain")); // contains space
        assert!(!is_valid_base_name("test_domain")); // contains underscore
    }

    // ── is_valid_fqdn ───────────────────────────────────────────────────

    #[test]
    fn test_valid_fqdns() {
        assert!(is_valid_fqdn("example.com"));
        assert!(is_valid_fqdn("test.co.uk"));
        assert!(is_valid_fqdn("sub.example.com"));
        assert!(is_valid_fqdn("a1.io")); // minimum valid
    }

    #[test]
    fn test_invalid_fqdns() {
        assert!(!is_valid_fqdn("example")); // no dot
        assert!(!is_valid_fqdn(".com")); // starts with dot
        assert!(!is_valid_fqdn("example.")); // ends with dot
        assert!(!is_valid_fqdn("-example.com")); // starts with hyphen
        assert!(!is_valid_fqdn("example.com-")); // ends with hyphen
        assert!(!is_valid_fqdn("ex.")); // part after dot is empty
        assert!(!is_valid_fqdn("a.b")); // too short (< 4 chars)
    }

    #[test]
    fn test_fqdn_too_long() {
        let long_domain = format!("{}.com", "a".repeat(250));
        assert!(!is_valid_fqdn(&long_domain)); // > 253 chars
    }

    #[test]
    fn test_fqdn_label_too_long() {
        let long_label = format!("{}.com", "a".repeat(64));
        assert!(!is_valid_fqdn(&long_label)); // label > 63 chars
    }

    #[test]
    fn test_fqdn_hyphen_in_label() {
        assert!(is_valid_fqdn("my-domain.com")); // hyphen in middle is ok
        assert!(!is_valid_fqdn("my--domain.-com")); // hyphen at start of label
    }

    #[test]
    fn test_fqdn_consecutive_dots() {
        assert!(!is_valid_fqdn("example..com")); // empty label between dots
    }
}
