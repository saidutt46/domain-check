//! Domain registry mappings and IANA bootstrap functionality.
//!
//! This module provides mappings from TLDs to their corresponding RDAP endpoints,
//! as well as dynamic discovery through the IANA bootstrap registry.

use crate::error::DomainCheckError;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Bootstrap registry cache for discovered RDAP endpoints and WHOIS servers.
///
/// This cache stores RDAP endpoints from the IANA bootstrap registry and
/// WHOIS server mappings discovered via IANA referral queries.
struct BootstrapCache {
    /// TLD -> RDAP endpoint URL (from IANA bootstrap)
    rdap_endpoints: HashMap<String, String>,
    /// TLD -> WHOIS server hostname (from IANA referral)
    whois_servers: HashMap<String, String>,
    /// TLDs known to have no RDAP endpoint (negative cache)
    no_rdap: HashSet<String>,
    /// Whether the full IANA bootstrap has been fetched
    rdap_loaded: bool,
    /// When the full bootstrap was last fetched
    last_fetch: Option<Instant>,
}

/// Bootstrap cache TTL: 24 hours (RDAP endpoints rarely change)
const BOOTSTRAP_TTL: Duration = Duration::from_secs(24 * 3600);

impl BootstrapCache {
    fn new() -> Self {
        Self {
            rdap_endpoints: HashMap::new(),
            whois_servers: HashMap::new(),
            no_rdap: HashSet::new(),
            rdap_loaded: false,
            last_fetch: None,
        }
    }

    fn is_stale(&self) -> bool {
        match self.last_fetch {
            Some(t) => t.elapsed() > BOOTSTRAP_TTL,
            None => true,
        }
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
        // Google TLDs (updated: rdap.nic.google no longer exists)
        ("app", "https://pubapi.registry.google/rdap/domain/"),
        ("dev", "https://pubapi.registry.google/rdap/domain/"),
        ("page", "https://pubapi.registry.google/rdap/domain/"),
        // CentralNic managed gTLDs
        ("xyz", "https://rdap.centralnic.com/xyz/domain/"),
        ("tech", "https://rdap.centralnic.com/tech/domain/"),
        ("online", "https://rdap.centralnic.com/online/domain/"),
        ("site", "https://rdap.centralnic.com/site/domain/"),
        ("website", "https://rdap.centralnic.com/website/domain/"),
        // Other popular gTLDs
        ("blog", "https://rdap.blog.fury.ca/rdap/domain/"),
        ("shop", "https://rdap.gmoregistry.net/rdap/domain/"),
        // Identity Digital managed TLDs
        ("ai", "https://rdap.identitydigital.services/rdap/domain/"), // Anguilla
        ("io", "https://rdap.identitydigital.services/rdap/domain/"), // British Indian Ocean Territory
        ("me", "https://rdap.identitydigital.services/rdap/domain/"), // Montenegro
        ("zone", "https://rdap.identitydigital.services/rdap/domain/"),
        (
            "digital",
            "https://rdap.identitydigital.services/rdap/domain/",
        ),
        // Country Code TLDs (ccTLDs) with working RDAP endpoints
        ("us", "https://rdap.nic.us/domain/"), // United States
        ("uk", "https://rdap.nominet.uk/domain/"), // United Kingdom
        ("de", "https://rdap.denic.de/domain/"), // Germany
        ("ca", "https://rdap.ca.fury.ca/rdap/domain/"), // Canada
        ("au", "https://rdap.cctld.au/rdap/domain/"), // Australia
        ("fr", "https://rdap.nic.fr/domain/"), // France
        ("nl", "https://rdap.sidn.nl/domain/"), // Netherlands
        ("br", "https://rdap.registro.br/domain/"), // Brazil
        ("in", "https://rdap.nixiregistry.in/rdap/domain/"), // India
        // Verisign managed ccTLDs
        ("tv", "https://rdap.nic.tv/domain/"), // Tuvalu
        ("cc", "https://tld-rdap.verisign.com/cc/v1/domain/"), // Cocos Islands
        // Specialty TLDs
        ("cloud", "https://rdap.registry.cloud/rdap/domain/"),
        // NOTE: co, eu, it, jp, es, cn removed — their RDAP endpoints are
        // defunct and no working alternatives found. These TLDs will fall
        // through to WHOIS fallback, which handles them correctly.
    ])
}

/// Get all TLDs that we have RDAP endpoints for.
///
/// Returns the union of hardcoded registry keys and bootstrap cache keys,
/// deduplicated and sorted alphabetically.
///
/// # Returns
///
/// Vector of TLD strings (e.g., ["com", "org", "net", ...]) sorted alphabetically.
pub fn get_all_known_tlds() -> Vec<String> {
    let registry = get_rdap_registry_map();
    let mut tld_set: HashSet<String> = registry.keys().map(|k| k.to_string()).collect();

    // Include bootstrap cache entries
    if let Ok(cache) = BOOTSTRAP_CACHE.lock() {
        for tld in cache.rdap_endpoints.keys() {
            tld_set.insert(tld.clone());
        }
    }

    let mut tlds: Vec<String> = tld_set.into_iter().collect();
    tlds.sort(); // Consistent ordering for user experience
    tlds
}

/// Get predefined TLD presets for common use cases.
///
/// This function provides curated TLD lists for common scenarios.
/// For custom preset support, use `get_preset_tlds_with_custom()`.
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
    let tlds: Option<Vec<&str>> = match preset.to_lowercase().as_str() {
        "startup" => Some(vec!["com", "org", "io", "ai", "tech", "app", "dev", "xyz"]),
        "enterprise" => Some(vec!["com", "org", "net", "info", "biz", "us"]),
        "country" => Some(vec!["us", "uk", "de", "fr", "ca", "au", "br", "in", "nl"]),
        "popular" => Some(vec![
            "com", "net", "org", "io", "ai", "app", "dev", "tech", "me", "co", "xyz",
        ]),
        "classic" => Some(vec!["com", "net", "org", "info", "biz"]),
        "tech" => Some(vec![
            "io",
            "ai",
            "app",
            "dev",
            "tech",
            "cloud",
            "software",
            "digital",
            "codes",
            "systems",
            "network",
            "solutions",
        ]),
        "creative" => Some(vec![
            "design",
            "art",
            "studio",
            "media",
            "photography",
            "film",
            "music",
            "gallery",
            "graphics",
            "ink",
        ]),
        "ecommerce" | "shopping" => Some(vec![
            "shop", "store", "market", "sale", "deals", "shopping", "buy", "bargains",
        ]),
        "finance" => Some(vec![
            "finance",
            "capital",
            "fund",
            "money",
            "investments",
            "insurance",
            "tax",
            "exchange",
            "trading",
        ]),
        "web" => Some(vec![
            "web", "site", "website", "online", "blog", "page", "wiki", "host", "email",
        ]),
        "trendy" => Some(vec![
            "xyz", "online", "site", "top", "icu", "fun", "space", "click", "website", "life",
            "world", "live", "today",
        ]),
        _ => None,
    };
    tlds.map(|v| v.into_iter().map(|s| s.to_string()).collect())
}

/// Get predefined TLD presets with custom preset support.
///
/// This function checks custom presets first, then falls back to built-in presets.
///
/// # Arguments
///
/// * `preset` - The preset name to look up
/// * `custom_presets` - Optional custom presets from config files
///
/// # Returns
///
/// Optional vector of TLD strings, None if preset doesn't exist.
///
/// # Examples
///
/// ```rust
/// use std::collections::HashMap;
/// use domain_check_lib::get_preset_tlds_with_custom;
///
/// let mut custom = HashMap::new();
/// custom.insert("my_preset".to_string(), vec!["com".to_string(), "dev".to_string()]);
///
/// let tlds = get_preset_tlds_with_custom("my_preset", Some(&custom)).unwrap();
/// assert_eq!(tlds, vec!["com", "dev"]);
/// ```
pub fn get_preset_tlds_with_custom(
    preset: &str,
    custom_presets: Option<&std::collections::HashMap<String, Vec<String>>>,
) -> Option<Vec<String>> {
    let preset_lower = preset.to_lowercase();

    // 1. Check custom presets first (highest precedence)
    if let Some(custom_map) = custom_presets {
        // Check both original case and lowercase
        if let Some(custom_tlds) = custom_map
            .get(preset)
            .or_else(|| custom_map.get(&preset_lower))
        {
            return Some(custom_tlds.clone());
        }
    }

    // 2. Fall back to built-in presets
    get_preset_tlds(&preset_lower)
}

/// Get available preset names.
///
/// Useful for CLI help text and validation.
///
/// # Returns
///
/// Vector of available preset names.
pub fn get_available_presets() -> Vec<&'static str> {
    vec![
        "classic",
        "country",
        "creative",
        "ecommerce",
        "enterprise",
        "finance",
        "popular",
        "startup",
        "tech",
        "trendy",
        "web",
    ]
}

/// Validate that all TLDs in a preset have hardcoded RDAP endpoints.
///
/// Returns true only if every TLD has a hardcoded RDAP endpoint in the
/// built-in registry. TLDs covered by bootstrap or WHOIS fallback will
/// return false here but still work at runtime.
///
/// # Arguments
///
/// * `preset_tlds` - TLD list to validate
///
/// # Returns
///
/// True if all TLDs have hardcoded RDAP endpoints, false otherwise.
#[allow(dead_code)]
pub fn validate_preset_tlds(preset_tlds: &[String]) -> bool {
    let registry = get_rdap_registry_map();
    preset_tlds
        .iter()
        .all(|tld| registry.contains_key(tld.as_str()))
}

/// Look up RDAP endpoint for a given TLD.
///
/// Lookup flow:
/// 1. Check hardcoded registry (32 TLDs) — instant, offline fallback
/// 2. Check bootstrap cache hit — O(1) HashMap lookup
/// 3. Check negative cache (no_rdap set) — skip network if TLD known to lack RDAP
/// 4. If cache empty or stale (24h): call fetch_full_bootstrap(), re-check
/// 5. If still not found after full fetch: add TLD to no_rdap set, return error
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

    // 1. Check built-in registry (instant, offline)
    let registry = get_rdap_registry_map();
    if let Some(endpoint) = registry.get(tld_lower.as_str()) {
        return Ok(endpoint.to_string());
    }

    // 2-3. Check bootstrap cache and negative cache
    {
        let cache = BOOTSTRAP_CACHE
            .lock()
            .map_err(|_| DomainCheckError::internal("Failed to acquire bootstrap cache lock"))?;

        // Check positive cache (not stale)
        if !cache.is_stale() {
            if let Some(endpoint) = cache.rdap_endpoints.get(&tld_lower) {
                return Ok(endpoint.clone());
            }
        }

        // Check negative cache (TLD known to have no RDAP)
        if cache.no_rdap.contains(&tld_lower) && !cache.is_stale() {
            return Err(DomainCheckError::bootstrap(
                &tld_lower,
                "TLD has no known RDAP endpoint",
            ));
        }
    }

    // 4. If bootstrap enabled, fetch full bootstrap and re-check
    if use_bootstrap {
        // Fetch if cache is empty or stale
        let needs_fetch = {
            let cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
                DomainCheckError::internal("Failed to acquire bootstrap cache lock")
            })?;
            !cache.rdap_loaded || cache.is_stale()
        };

        if needs_fetch {
            fetch_full_bootstrap().await?;
        }

        // Re-check after fetch
        let cache = BOOTSTRAP_CACHE
            .lock()
            .map_err(|_| DomainCheckError::internal("Failed to acquire bootstrap cache lock"))?;

        if let Some(endpoint) = cache.rdap_endpoints.get(&tld_lower) {
            return Ok(endpoint.clone());
        }

        // 5. Still not found — add to negative cache and return error
        drop(cache);
        {
            let mut cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
                DomainCheckError::internal("Failed to acquire bootstrap cache lock")
            })?;
            cache.no_rdap.insert(tld_lower.clone());
        }

        Err(DomainCheckError::bootstrap(
            &tld_lower,
            "TLD not found in IANA bootstrap registry",
        ))
    } else {
        Err(DomainCheckError::bootstrap(
            &tld_lower,
            "No known RDAP endpoint and bootstrap disabled",
        ))
    }
}

/// Fetch the full IANA bootstrap registry and populate the cache.
///
/// Instead of fetching per-TLD, this downloads the complete IANA RDAP bootstrap
/// JSON and parses all service entries at once. Much more efficient for bulk
/// operations and provides coverage for ~1,180 TLDs.
async fn fetch_full_bootstrap() -> Result<(), DomainCheckError> {
    const BOOTSTRAP_URL: &str = "https://data.iana.org/rdap/dns.json";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| {
            DomainCheckError::network_with_source("Failed to create HTTP client", e.to_string())
        })?;

    let response = client.get(BOOTSTRAP_URL).send().await.map_err(|e| {
        DomainCheckError::bootstrap("*", format!("Failed to fetch bootstrap registry: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(DomainCheckError::bootstrap(
            "*",
            format!("Bootstrap registry returned HTTP {}", response.status()),
        ));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        DomainCheckError::bootstrap("*", format!("Failed to parse bootstrap JSON: {}", e))
    })?;

    // Validate structure
    let services = json
        .get("services")
        .and_then(|s| s.as_array())
        .ok_or_else(|| {
            DomainCheckError::bootstrap(
                "*",
                "Invalid bootstrap JSON: missing or invalid 'services' array",
            )
        })?;

    let mut endpoints: HashMap<String, String> = HashMap::new();

    for service in services {
        if let Some(service_array) = service.as_array() {
            if service_array.len() >= 2 {
                // Get the endpoint URL(s)
                let url = service_array[1]
                    .as_array()
                    .and_then(|urls| urls.first())
                    .and_then(|u| u.as_str());

                if let Some(url) = url {
                    let endpoint = format!("{}/domain/", url.trim_end_matches('/'));

                    // Get all TLDs served by this endpoint
                    if let Some(tlds) = service_array[0].as_array() {
                        for t in tlds {
                            if let Some(tld_str) = t.as_str() {
                                endpoints.insert(tld_str.to_lowercase(), endpoint.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Update cache atomically
    let mut cache = BOOTSTRAP_CACHE
        .lock()
        .map_err(|_| DomainCheckError::internal("Failed to acquire bootstrap cache lock"))?;

    cache.rdap_endpoints = endpoints;
    cache.rdap_loaded = true;
    cache.last_fetch = Some(Instant::now());
    cache.no_rdap.clear(); // Reset negative cache on fresh fetch

    Ok(())
}

/// Pre-warm the bootstrap cache by fetching the full IANA registry.
///
/// Call this before bulk operations (e.g., `--all` mode) to ensure all ~1,180
/// TLDs are available without per-TLD network requests.
///
/// This is safe to call multiple times — subsequent calls are no-ops if the
/// cache is still fresh (within the 24-hour TTL).
pub async fn initialize_bootstrap() -> Result<(), DomainCheckError> {
    let needs_fetch = {
        let cache = BOOTSTRAP_CACHE
            .lock()
            .map_err(|_| DomainCheckError::internal("Failed to acquire bootstrap cache lock"))?;
        !cache.rdap_loaded || cache.is_stale()
    };

    if needs_fetch {
        fetch_full_bootstrap().await?;
    }

    Ok(())
}

/// Cache a discovered WHOIS server for a TLD.
pub fn cache_whois_server(tld: &str, server: &str) -> Result<(), DomainCheckError> {
    let mut cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
        DomainCheckError::internal("Failed to acquire bootstrap cache lock for writing")
    })?;

    cache
        .whois_servers
        .insert(tld.to_lowercase(), server.to_string());
    Ok(())
}

/// Look up a cached WHOIS server for a TLD.
///
/// Checks the bootstrap cache for a previously discovered WHOIS server.
/// If not cached, the caller should use `discover_whois_server()` from
/// the whois module and cache the result.
///
/// # Arguments
///
/// * `tld` - The TLD to look up (e.g., "com", "co")
///
/// # Returns
///
/// The WHOIS server hostname if cached, or None.
pub fn get_cached_whois_server(tld: &str) -> Option<String> {
    let cache = BOOTSTRAP_CACHE.lock().ok()?;
    let server = cache.whois_servers.get(&tld.to_lowercase())?;
    if server.is_empty() {
        None // Empty string means "no server found" (negative cache)
    } else {
        Some(server.clone())
    }
}

/// Check if a TLD has been negatively cached for WHOIS (no server found).
pub fn is_whois_negatively_cached(tld: &str) -> bool {
    if let Ok(cache) = BOOTSTRAP_CACHE.lock() {
        matches!(cache.whois_servers.get(&tld.to_lowercase()), Some(s) if s.is_empty())
    } else {
        false
    }
}

/// Get the WHOIS server for a TLD, using cache with IANA referral discovery fallback.
///
/// Lookup flow:
/// 1. Check cache for previously discovered server
/// 2. If miss and not negatively cached, discover via IANA referral
/// 3. Cache result (empty string for "no server found" to avoid re-querying)
///
/// # Arguments
///
/// * `tld` - The TLD to look up
///
/// # Returns
///
/// The WHOIS server hostname, or None if no server exists for this TLD.
pub async fn get_whois_server(tld: &str) -> Option<String> {
    let tld_lower = tld.to_lowercase();

    // Check positive cache
    if let Some(server) = get_cached_whois_server(&tld_lower) {
        return Some(server);
    }

    // Check negative cache
    if is_whois_negatively_cached(&tld_lower) {
        return None;
    }

    // Discover via IANA referral
    match crate::protocols::whois::discover_whois_server(&tld_lower).await {
        Some(server) => {
            let _ = cache_whois_server(&tld_lower, &server);
            Some(server)
        }
        None => {
            // Cache empty string as negative result
            let _ = cache_whois_server(&tld_lower, "");
            None
        }
    }
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

    cache.rdap_endpoints.clear();
    cache.whois_servers.clear();
    cache.no_rdap.clear();
    cache.rdap_loaded = false;
    cache.last_fetch = None;
    Ok(())
}

/// Get bootstrap cache statistics (useful for debugging).
#[allow(dead_code)]
pub fn get_bootstrap_cache_stats() -> Result<(usize, bool), DomainCheckError> {
    let cache = BOOTSTRAP_CACHE.lock().map_err(|_| {
        DomainCheckError::internal("Failed to acquire bootstrap cache lock for stats")
    })?;

    Ok((cache.rdap_endpoints.len(), cache.is_stale()))
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

    #[test]
    fn test_all_endpoints_are_valid_https_urls() {
        let registry = get_rdap_registry_map();
        for (tld, endpoint) in &registry {
            assert!(
                endpoint.starts_with("https://"),
                "Endpoint for '{}' must use HTTPS: {}",
                tld,
                endpoint
            );
            assert!(
                endpoint.ends_with("/domain/"),
                "Endpoint for '{}' must end with /domain/: {}",
                tld,
                endpoint
            );
        }
    }

    #[test]
    fn test_bootstrap_cache_new() {
        let cache = BootstrapCache::new();
        assert!(!cache.rdap_loaded);
        assert!(cache.last_fetch.is_none());
        assert!(cache.rdap_endpoints.is_empty());
        assert!(cache.whois_servers.is_empty());
        assert!(cache.no_rdap.is_empty());
        assert!(cache.is_stale());
    }

    #[test]
    fn test_whois_server_caching() {
        clear_bootstrap_cache().unwrap();

        // Cache a server
        cache_whois_server("com", "whois.verisign-grs.com").unwrap();
        assert_eq!(
            get_cached_whois_server("com"),
            Some("whois.verisign-grs.com".to_string())
        );

        // Cache negative result
        cache_whois_server("fake", "").unwrap();
        assert_eq!(get_cached_whois_server("fake"), None);
        assert!(is_whois_negatively_cached("fake"));

        clear_bootstrap_cache().unwrap();
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
        assert!(tlds.contains(&"nl".to_string()));
    }

    #[test]
    fn test_invalid_preset() {
        assert!(get_preset_tlds("invalid").is_none());
        assert!(get_preset_tlds("").is_none());
    }

    #[test]
    fn test_available_presets() {
        let presets = get_available_presets();
        assert_eq!(presets.len(), 11);
        assert!(presets.contains(&"startup"));
        assert!(presets.contains(&"enterprise"));
        assert!(presets.contains(&"country"));
        assert!(presets.contains(&"popular"));
        assert!(presets.contains(&"classic"));
        assert!(presets.contains(&"tech"));
        assert!(presets.contains(&"creative"));
        assert!(presets.contains(&"ecommerce"));
        assert!(presets.contains(&"finance"));
        assert!(presets.contains(&"web"));
        assert!(presets.contains(&"trendy"));
    }

    #[test]
    fn test_validate_preset_tlds() {
        // Core presets (startup, enterprise, country, popular, classic) should
        // have hardcoded RDAP endpoints for offline operation
        let core_presets = ["startup", "enterprise", "country", "classic"];
        for preset_name in &core_presets {
            let tlds = get_preset_tlds(preset_name).unwrap();
            assert!(
                validate_preset_tlds(&tlds),
                "Core preset '{}' contains TLDs without hardcoded RDAP endpoints",
                preset_name
            );
        }
    }

    #[test]
    fn test_all_presets_non_empty() {
        for preset_name in get_available_presets() {
            let tlds = get_preset_tlds(preset_name).unwrap();
            assert!(
                !tlds.is_empty(),
                "Preset '{}' should not be empty",
                preset_name
            );
        }
    }

    #[test]
    fn test_ecommerce_alias() {
        assert_eq!(get_preset_tlds("ecommerce"), get_preset_tlds("shopping"));
    }

    #[test]
    fn test_preset_tlds_subset_of_known() {
        // Only validate core presets against hardcoded TLDs
        // (extended presets require bootstrap which isn't available in unit tests)
        let core_presets = ["startup", "enterprise", "country", "classic"];
        let all_tlds = get_all_known_tlds();

        for preset_name in &core_presets {
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
