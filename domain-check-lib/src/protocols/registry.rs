//! Domain registry mappings and IANA bootstrap functionality.
//!
//! This module provides mappings from TLDs to their corresponding RDAP endpoints,
//! as well as dynamic discovery through the IANA bootstrap registry.

use crate::error::DomainCheckError;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Bootstrap registry cache for discovered RDAP endpoints
struct BootstrapCache {
    endpoints: HashMap<String, String>,
    last_update: Instant,
}

impl BootstrapCache {
    fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    fn get(&self, tld: &str) -> Option<String> {
        self.endpoints.get(tld).cloned()
    }

    fn insert(&mut self, tld: String, endpoint: String) {
        self.endpoints.insert(tld, endpoint);
        self.last_update = Instant::now();
    }

    fn is_stale(&self) -> bool {
        // Cache expires after 1 hour
        self.last_update.elapsed() > Duration::from_secs(3600)
    }
}

// Global bootstrap cache using lazy_static
lazy_static::lazy_static! {
    static ref BOOTSTRAP_CACHE: Mutex<BootstrapCache> = Mutex::new(BootstrapCache::new());
}

/// Get the built-in RDAP registry mappings.
///
/// This function returns a map of TLD strings to their corresponding RDAP endpoint URLs.
/// These mappings are based on known registry endpoints and are updated periodically.
///
/// # Returns
///
/// A HashMap mapping TLD strings (like "com", "org") to RDAP endpoint base URLs.
pub fn get_rdap_registry_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        // Popular gTLDs (Generic Top-Level Domains)
        ("com", "https://rdap.verisign.com/com/v1/domain/"),
        ("net", "https://rdap.verisign.com/net/v1/domain/"),
        (
            "org",
            "https://rdap.publicinterestregistry.org/rdap/domain/",
        ),
        ("info", "https://rdap.identitydigital.services/rdap/domain/"),
        ("biz", "https://rdap.nic.biz/domain/"),
        // Google TLDs
        ("app", "https://rdap.nic.google/domain/"),
        ("dev", "https://rdap.nic.google/domain/"),
        ("page", "https://rdap.nic.google/domain/"),
        // Other popular gTLDs
        ("blog", "https://rdap.nic.blog/domain/"),
        ("shop", "https://rdap.nic.shop/domain/"),
        ("xyz", "https://rdap.nic.xyz/domain/"),
        ("tech", "https://rdap.nic.tech/domain/"),
        ("online", "https://rdap.nic.online/domain/"),
        ("site", "https://rdap.nic.site/domain/"),
        ("website", "https://rdap.nic.website/domain/"),
        // Country Code TLDs (ccTLDs)
        ("io", "https://rdap.identitydigital.services/rdap/domain/"), // British Indian Ocean Territory
        ("ai", "https://rdap.nic.ai/domain/"),                        // Anguilla
        ("co", "https://rdap.nic.co/domain/"),                        // Colombia
        ("me", "https://rdap.nic.me/domain/"),                        // Montenegro
        ("us", "https://rdap.nic.us/domain/"),                        // United States
        ("uk", "https://rdap.nominet.uk/domain/"),                    // United Kingdom
        ("eu", "https://rdap.eu.org/domain/"),                        // European Union
        ("de", "https://rdap.denic.de/domain/"),                      // Germany
        ("ca", "https://rdap.cira.ca/domain/"),                       // Canada
        ("au", "https://rdap.auda.org.au/domain/"),                   // Australia
        ("fr", "https://rdap.nic.fr/domain/"),                        // France
        ("es", "https://rdap.nic.es/domain/"),                        // Spain
        ("it", "https://rdap.nic.it/domain/"),                        // Italy
        ("nl", "https://rdap.domain-registry.nl/domain/"),            // Netherlands
        ("jp", "https://rdap.jprs.jp/domain/"),                       // Japan
        ("br", "https://rdap.registro.br/domain/"),                   // Brazil
        ("in", "https://rdap.registry.in/domain/"),                   // India
        ("cn", "https://rdap.cnnic.cn/domain/"),                      // China
        // Verisign managed ccTLDs
        ("tv", "https://rdap.verisign.com/tv/v1/domain/"), // Tuvalu
        ("cc", "https://rdap.verisign.com/cc/v1/domain/"), // Cocos Islands
        // Specialty TLDs
        ("zone", "https://rdap.nic.zone/domain/"),
        ("cloud", "https://rdap.nic.cloud/domain/"),
        ("digital", "https://rdap.nic.digital/domain/"),
    ])
}

// Add these functions after line 81 in domain-check-lib/src/protocols/registry.rs

/// Get all TLDs that we have RDAP endpoints for.
///
/// This function extracts TLD knowledge from our built-in registry mappings,
/// providing a comprehensive list for the --all flag functionality.
///
/// # Returns
///
/// Vector of TLD strings (e.g., ["com", "org", "net", ...]) sorted alphabetically.
pub fn get_all_known_tlds() -> Vec<String> {
    let registry = get_rdap_registry_map();
    let mut tlds: Vec<String> = registry.keys().map(|k| k.to_string()).collect();
    tlds.sort(); // Consistent ordering for user experience
    tlds
}

/// Get predefined TLD presets for common use cases.
///
/// This function provides curated TLD lists that cover the most common
/// domain checking scenarios without overwhelming users.
///
/// # Arguments
///
/// * `preset` - The preset name ("startup", "enterprise", "country")
///
/// # Returns
///
/// Optional vector of TLD strings, None if preset doesn't exist.
///
/// # Examples
///
/// ```rust
/// use domain_check_lib::get_preset_tlds;
///
/// let startup_tlds = get_preset_tlds("startup").unwrap();
/// assert!(startup_tlds.contains(&"io".to_string()));
/// ```
pub fn get_preset_tlds(preset: &str) -> Option<Vec<String>> {
    match preset.to_lowercase().as_str() {
        "startup" => Some(vec![
            "com".to_string(),
            "org".to_string(),
            "io".to_string(),
            "ai".to_string(),
            "tech".to_string(),
            "app".to_string(),
            "dev".to_string(),
            "xyz".to_string(),
        ]),
        "enterprise" => Some(vec![
            "com".to_string(),
            "org".to_string(),
            "net".to_string(),
            "info".to_string(),
            "biz".to_string(),
            "us".to_string(),
        ]),
        "country" => Some(vec![
            "us".to_string(),
            "uk".to_string(),
            "de".to_string(),
            "fr".to_string(),
            "ca".to_string(),
            "au".to_string(),
            "jp".to_string(),
            "br".to_string(),
            "in".to_string(),
        ]),
        _ => None,
    }
}

/// Get available preset names.
///
/// Useful for CLI help text and validation.
///
/// # Returns
///
/// Vector of available preset names.
pub fn get_available_presets() -> Vec<&'static str> {
    vec!["startup", "enterprise", "country"]
}

/// Validate that all TLDs in a preset have RDAP endpoints.
///
/// This ensures preset TLDs can actually be checked via our registry.
/// Used internally and for testing.
///
/// # Arguments
///
/// * `preset_tlds` - TLD list to validate
///
/// # Returns
///
/// True if all TLDs have known RDAP endpoints, false otherwise.
#[allow(dead_code)]
pub fn validate_preset_tlds(preset_tlds: &[String]) -> bool {
    let registry = get_rdap_registry_map();
    preset_tlds
        .iter()
        .all(|tld| registry.contains_key(tld.as_str()))
}

/// Look up RDAP endpoint for a given TLD.
///
/// First checks the built-in registry, then checks the bootstrap cache,
/// and optionally discovers new endpoints via IANA bootstrap.
///
/// # Arguments
///
/// * `tld` - The top-level domain to look up (e.g., "com", "org")
/// * `use_bootstrap` - Whether to use IANA bootstrap for unknown TLDs
///
/// # Returns
///
/// The RDAP endpoint URL if found, or an error if not available.
pub async fn get_rdap_endpoint(tld: &str, use_bootstrap: bool) -> Result<String, DomainCheckError> {
    let tld_lower = tld.to_lowercase();

    // First, check built-in registry
    let registry = get_rdap_registry_map();
    if let Some(endpoint) = registry.get(tld_lower.as_str()) {
        return Ok(endpoint.to_string());
    }

    // Check bootstrap cache
    {
        let cache = BOOTSTRAP_CACHE
            .lock()
            .map_err(|_| DomainCheckError::internal("Failed to acquire bootstrap cache lock"))?;

        if !cache.is_stale() {
            if let Some(endpoint) = cache.get(&tld_lower) {
                return Ok(endpoint);
            }
        }
    }

    // If bootstrap is enabled, try to discover the endpoint
    if use_bootstrap {
        discover_rdap_endpoint(&tld_lower).await
    } else {
        Err(DomainCheckError::bootstrap(
            &tld_lower,
            "No known RDAP endpoint and bootstrap disabled",
        ))
    }
}

/// Discover RDAP endpoint for a TLD using IANA bootstrap registry.
///
/// This function queries the IANA bootstrap registry to find the RDAP endpoint
/// for TLDs that are not in our built-in mappings.
///
/// # Arguments
///
/// * `tld` - The TLD to discover an endpoint for
///
/// # Returns
///
/// The discovered RDAP endpoint URL, or an error if discovery fails.
async fn discover_rdap_endpoint(tld: &str) -> Result<String, DomainCheckError> {
    const BOOTSTRAP_URL: &str = "https://data.iana.org/rdap/dns.json";

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| {
            DomainCheckError::network_with_source("Failed to create HTTP client", e.to_string())
        })?;

    // Fetch bootstrap registry
    let response = client.get(BOOTSTRAP_URL).send().await.map_err(|e| {
        DomainCheckError::bootstrap(tld, format!("Failed to fetch bootstrap registry: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(DomainCheckError::bootstrap(
            tld,
            format!("Bootstrap registry returned HTTP {}", response.status()),
        ));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        DomainCheckError::bootstrap(tld, format!("Failed to parse bootstrap JSON: {}", e))
    })?;

    // Parse the bootstrap registry format
    if let Some(services) = json.get("services").and_then(|s| s.as_array()) {
        for service in services {
            if let Some(service_array) = service.as_array() {
                if service_array.len() >= 2 {
                    // Check if this service handles our TLD
                    if let Some(tlds) = service_array[0].as_array() {
                        for t in tlds {
                            if let Some(t_str) = t.as_str() {
                                if t_str.to_lowercase() == tld.to_lowercase() {
                                    // Found our TLD, get the endpoint
                                    if let Some(urls) = service_array[1].as_array() {
                                        if let Some(url) = urls.first().and_then(|u| u.as_str()) {
                                            let endpoint =
                                                format!("{}/domain/", url.trim_end_matches('/'));

                                            // Cache the discovered endpoint
                                            cache_discovered_endpoint(tld, &endpoint)?;

                                            return Ok(endpoint);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(DomainCheckError::bootstrap(
        tld,
        "TLD not found in IANA bootstrap registry",
    ))
}

/// Cache a discovered RDAP endpoint for future use.
fn cache_discovered_endpoint(tld: &str, endpoint: &str) -> Result<(), DomainCheckError> {
    let mut cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
        DomainCheckError::internal("Failed to acquire bootstrap cache lock for writing")
    })?;

    cache.insert(tld.to_string(), endpoint.to_string());
    Ok(())
}

/// Extract TLD from a domain name.
///
/// Handles both simple TLDs (example.com -> "com") and multi-level TLDs
/// (example.co.uk -> "co.uk", though this function will return "uk").
///
/// # Arguments
///
/// * `domain` - The domain name to extract TLD from
///
/// # Returns
///
/// The TLD string, or an error if the domain format is invalid.
pub fn extract_tld(domain: &str) -> Result<String, DomainCheckError> {
    let parts: Vec<&str> = domain.split('.').collect();

    if parts.len() < 2 {
        return Err(DomainCheckError::invalid_domain(
            domain,
            "Domain must contain at least one dot",
        ));
    }

    // Return the last part as TLD
    // Note: This is simplified and doesn't handle multi-level TLDs like .co.uk
    // For production use, consider using a library like publicsuffix
    Ok(parts.last().unwrap().to_lowercase())
}

/// Clear the bootstrap cache (useful for testing).
#[allow(dead_code)]
pub fn clear_bootstrap_cache() -> Result<(), DomainCheckError> {
    let mut cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
        DomainCheckError::internal("Failed to acquire bootstrap cache lock for clearing")
    })?;

    cache.endpoints.clear();
    cache.last_update = Instant::now();
    Ok(())
}

/// Get bootstrap cache statistics (useful for debugging).
#[allow(dead_code)]
pub fn get_bootstrap_cache_stats() -> Result<(usize, bool), DomainCheckError> {
    let cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
        DomainCheckError::internal("Failed to acquire bootstrap cache lock for stats")
    })?;

    Ok((cache.endpoints.len(), cache.is_stale()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tld() {
        assert_eq!(extract_tld("example.com").unwrap(), "com");
        assert_eq!(extract_tld("test.org").unwrap(), "org");
        assert_eq!(extract_tld("sub.example.com").unwrap(), "com");
        assert!(extract_tld("invalid").is_err());
        assert!(extract_tld("").is_err());
    }

    #[test]
    fn test_registry_map_contains_common_tlds() {
        let registry = get_rdap_registry_map();
        assert!(registry.contains_key("com"));
        assert!(registry.contains_key("org"));
        assert!(registry.contains_key("net"));
        assert!(registry.contains_key("io"));
    }

    #[tokio::test]
    async fn test_get_rdap_endpoint_builtin() {
        let endpoint = get_rdap_endpoint("com", false).await.unwrap();
        assert!(endpoint.contains("verisign.com"));
    }

    #[tokio::test]
    async fn test_get_rdap_endpoint_unknown_no_bootstrap() {
        let result = get_rdap_endpoint("unknowntld123", false).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn test_get_all_known_tlds() {
        let tlds = get_all_known_tlds();

        // Should have our expected core TLDs
        assert!(tlds.len() >= 30);
        assert!(tlds.contains(&"com".to_string()));
        assert!(tlds.contains(&"org".to_string()));
        assert!(tlds.contains(&"io".to_string()));
        assert!(tlds.contains(&"ai".to_string()));

        // Should be sorted for consistent UX
        let mut sorted_tlds = tlds.clone();
        sorted_tlds.sort();
        assert_eq!(tlds, sorted_tlds);
    }

    #[test]
    fn test_startup_preset() {
        let tlds = get_preset_tlds("startup").unwrap();

        assert_eq!(tlds.len(), 8);
        assert!(tlds.contains(&"com".to_string()));
        assert!(tlds.contains(&"io".to_string()));
        assert!(tlds.contains(&"ai".to_string()));
        assert!(tlds.contains(&"tech".to_string()));

        // Case insensitive
        assert_eq!(get_preset_tlds("STARTUP"), get_preset_tlds("startup"));
    }

    #[test]
    fn test_enterprise_preset() {
        let tlds = get_preset_tlds("enterprise").unwrap();

        assert_eq!(tlds.len(), 6);
        assert!(tlds.contains(&"com".to_string()));
        assert!(tlds.contains(&"org".to_string()));
        assert!(tlds.contains(&"biz".to_string()));
    }

    #[test]
    fn test_country_preset() {
        let tlds = get_preset_tlds("country").unwrap();

        assert_eq!(tlds.len(), 9);
        assert!(tlds.contains(&"us".to_string()));
        assert!(tlds.contains(&"uk".to_string()));
        assert!(tlds.contains(&"de".to_string()));
    }

    #[test]
    fn test_invalid_preset() {
        assert!(get_preset_tlds("invalid").is_none());
        assert!(get_preset_tlds("").is_none());
    }

    #[test]
    fn test_available_presets() {
        let presets = get_available_presets();
        assert_eq!(presets.len(), 3);
        assert!(presets.contains(&"startup"));
        assert!(presets.contains(&"enterprise"));
        assert!(presets.contains(&"country"));
    }

    #[test]
    fn test_validate_preset_tlds() {
        // All preset TLDs should have RDAP endpoints
        for preset_name in get_available_presets() {
            let tlds = get_preset_tlds(preset_name).unwrap();
            assert!(
                validate_preset_tlds(&tlds),
                "Preset '{}' contains TLDs without RDAP endpoints",
                preset_name
            );
        }
    }

    #[test]
    fn test_preset_tlds_subset_of_known() {
        let all_tlds = get_all_known_tlds();

        for preset_name in get_available_presets() {
            let preset_tlds = get_preset_tlds(preset_name).unwrap();
            for tld in preset_tlds {
                assert!(
                    all_tlds.contains(&tld),
                    "Preset '{}' contains unknown TLD: {}",
                    preset_name,
                    tld
                );
            }
        }
    }
}
