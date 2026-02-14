// domain-check-lib/tests/integration.rs

//! Integration tests for domain-check-lib exports and core functionality

use domain_check_lib::{
    get_all_known_tlds, get_available_presets, get_preset_tlds, get_whois_server,
    initialize_bootstrap,
};

#[test]
fn test_library_exports_work() {
    // Test that all exported functions are accessible and work

    // Test get_all_known_tlds export
    let all_tlds = get_all_known_tlds();
    assert!(!all_tlds.is_empty());
    assert!(all_tlds.contains(&"com".to_string()));
    assert!(all_tlds.contains(&"org".to_string()));

    // Test get_preset_tlds export
    let startup_tlds = get_preset_tlds("startup").unwrap();
    assert!(!startup_tlds.is_empty());
    assert!(startup_tlds.contains(&"io".to_string()));
    assert!(startup_tlds.contains(&"ai".to_string()));

    // Test get_available_presets export
    let presets = get_available_presets();
    assert_eq!(presets.len(), 11);
    assert!(presets.contains(&"startup"));
    assert!(presets.contains(&"enterprise"));
    assert!(presets.contains(&"country"));
    assert!(presets.contains(&"popular"));
    assert!(presets.contains(&"tech"));
    assert!(presets.contains(&"creative"));
    assert!(presets.contains(&"ecommerce"));
    assert!(presets.contains(&"finance"));
    assert!(presets.contains(&"web"));
    assert!(presets.contains(&"trendy"));
    assert!(presets.contains(&"classic"));
}

#[test]
fn test_core_preset_tlds_are_subset_of_all_tlds() {
    let all_tlds = get_all_known_tlds();
    // Only validate core presets against hardcoded TLDs.
    // Extended presets (tech, creative, etc.) include TLDs that require
    // bootstrap to resolve — they work at runtime but aren't in the
    // hardcoded list.
    let core_presets = ["startup", "enterprise", "country", "classic"];

    for preset_name in &core_presets {
        let preset_tlds = get_preset_tlds(preset_name).unwrap();
        for tld in preset_tlds {
            assert!(
                all_tlds.contains(&tld),
                "Core preset '{}' contains TLD '{}' not in all_known_tlds",
                preset_name,
                tld
            );
        }
    }
}

#[test]
fn test_all_known_tlds_sorted() {
    let tlds = get_all_known_tlds();
    let mut sorted_tlds = tlds.clone();
    sorted_tlds.sort();

    assert_eq!(tlds, sorted_tlds, "TLDs should be returned in sorted order");
}

#[test]
fn test_preset_tlds_case_insensitive() {
    assert_eq!(get_preset_tlds("startup"), get_preset_tlds("STARTUP"));
    assert_eq!(get_preset_tlds("enterprise"), get_preset_tlds("ENTERPRISE"));
    assert_eq!(get_preset_tlds("country"), get_preset_tlds("COUNTRY"));
    assert_eq!(get_preset_tlds("popular"), get_preset_tlds("POPULAR"));
    assert_eq!(get_preset_tlds("tech"), get_preset_tlds("TECH"));
    assert_eq!(get_preset_tlds("creative"), get_preset_tlds("CREATIVE"));
    assert_eq!(get_preset_tlds("ecommerce"), get_preset_tlds("ECOMMERCE"));
    assert_eq!(get_preset_tlds("finance"), get_preset_tlds("FINANCE"));
    assert_eq!(get_preset_tlds("web"), get_preset_tlds("WEB"));
    assert_eq!(get_preset_tlds("trendy"), get_preset_tlds("TRENDY"));
    assert_eq!(get_preset_tlds("classic"), get_preset_tlds("CLASSIC"));
}

#[test]
fn test_preset_tlds_invalid_returns_none() {
    assert!(get_preset_tlds("nonexistent").is_none());
    assert!(get_preset_tlds("").is_none());
    assert!(get_preset_tlds("invalid_preset_name").is_none());
}

/// Smoke test: google.com must always be reported as taken.
/// This is the single most critical invariant for a domain availability checker.
#[tokio::test]
async fn test_known_taken_domain_google_com() {
    use domain_check_lib::DomainChecker;

    let checker = DomainChecker::new();
    let result = checker.check_domain("google.com").await.unwrap();
    assert_eq!(
        result.available,
        Some(false),
        "google.com must be reported as TAKEN"
    );
}

// ============================================================
// Bootstrap bulk fetch tests
// ============================================================

/// Test that initialize_bootstrap() fetches >1000 TLD entries from IANA.
/// This hits the network so it's marked #[ignore] for CI unless explicitly run.
#[tokio::test]
#[ignore]
async fn test_fetch_full_bootstrap_returns_over_1000_entries() {
    initialize_bootstrap().await.unwrap();

    let tlds = get_all_known_tlds();
    assert!(
        tlds.len() > 1000,
        "Expected >1000 TLDs after bootstrap, got {}",
        tlds.len()
    );
}

/// Test that after bootstrap, get_all_known_tlds() includes TLDs not in the hardcoded list.
#[tokio::test]
#[ignore]
async fn test_bootstrap_adds_non_hardcoded_tlds() {
    initialize_bootstrap().await.unwrap();

    let tlds = get_all_known_tlds();
    // .museum is a real TLD that's not in the 32 hardcoded ones
    assert!(
        tlds.contains(&"museum".to_string()),
        "Bootstrap should include .museum TLD"
    );
    // .travel is another uncommon one
    assert!(
        tlds.contains(&"travel".to_string()),
        "Bootstrap should include .travel TLD"
    );
}

/// Test that initialize_bootstrap() is idempotent (second call is a no-op).
#[tokio::test]
#[ignore]
async fn test_initialize_bootstrap_idempotent() {
    initialize_bootstrap().await.unwrap();
    let count1 = get_all_known_tlds().len();

    // Second call should be a no-op (cache is fresh)
    initialize_bootstrap().await.unwrap();
    let count2 = get_all_known_tlds().len();

    assert_eq!(
        count1, count2,
        "Second bootstrap call should not change TLD count"
    );
}

/// Test that bootstrap results are sorted and deduplicated with hardcoded entries.
#[tokio::test]
#[ignore]
async fn test_bootstrap_results_sorted_and_deduplicated() {
    initialize_bootstrap().await.unwrap();

    let tlds = get_all_known_tlds();

    // Should be sorted
    let mut sorted = tlds.clone();
    sorted.sort();
    assert_eq!(tlds, sorted, "TLDs must be sorted after bootstrap");

    // Should contain hardcoded TLDs (no duplication)
    let com_count = tlds.iter().filter(|t| t.as_str() == "com").count();
    assert_eq!(
        com_count, 1,
        "\"com\" should appear exactly once (deduplicated)"
    );
}

// ============================================================
// WHOIS server discovery tests
// ============================================================

/// Test WHOIS discovery for multiple TLDs and caching.
///
/// This is a single sequential test because whois.iana.org:43 rate-limits
/// concurrent TCP connections. Running these as separate parallel tests
/// causes connection drops.
#[tokio::test]
#[ignore]
async fn test_whois_discovery_and_caching() {
    // .com should return whois.verisign-grs.com
    let com_server = get_whois_server("com").await;
    assert_eq!(
        com_server,
        Some("whois.verisign-grs.com".to_string()),
        ".com WHOIS server should be whois.verisign-grs.com"
    );

    // .org should have a WHOIS server
    let org_server = get_whois_server("org").await;
    assert!(
        org_server.is_some(),
        ".org should have a WHOIS server via IANA referral"
    );

    // .co (not in hardcoded RDAP) should have WHOIS
    let co_server = get_whois_server("co").await;
    assert!(
        co_server.is_some(),
        ".co should have a WHOIS server (it was removed from hardcoded RDAP)"
    );

    // Caching: second call for .com should return the same result (from cache)
    let com_server_cached = get_whois_server("com").await;
    assert_eq!(
        com_server, com_server_cached,
        "Cached result should match first result"
    );
}

// ============================================================
// End-to-end: check domain on non-hardcoded TLD
// ============================================================

/// Test checking a domain on a TLD not in the hardcoded 32 (e.g., .museum).
/// With bootstrap enabled (default), this should work via IANA bootstrap.
#[tokio::test]
#[ignore]
async fn test_check_domain_on_non_hardcoded_tld() {
    use domain_check_lib::{CheckConfig, DomainChecker};

    let config = CheckConfig::default().with_bootstrap(true);
    let checker = DomainChecker::with_config(config);

    // google.museum is a real domain — check should return a definitive result
    let result = checker.check_domain("nic.museum").await;
    assert!(
        result.is_ok(),
        "Should be able to check .museum domain via bootstrap: {:?}",
        result.err()
    );
    let result = result.unwrap();
    assert!(
        result.available.is_some(),
        "Should get a definitive availability result for nic.museum"
    );
}

/// Test that bootstrap-disabled mode falls back gracefully for unknown TLDs.
#[tokio::test]
async fn test_check_domain_no_bootstrap_unknown_tld_falls_back_to_whois() {
    use domain_check_lib::{CheckConfig, DomainChecker};

    let config = CheckConfig::default()
        .with_bootstrap(false)
        .with_whois_fallback(true);
    let checker = DomainChecker::with_config(config);

    // .museum is not in hardcoded RDAP and bootstrap is off,
    // so RDAP fails and WHOIS fallback should handle it
    let result = checker.check_domain("nic.museum").await;
    // With WHOIS fallback, we should get some result (not a hard error)
    assert!(
        result.is_ok(),
        "WHOIS fallback should handle unknown TLDs gracefully"
    );
}

/// Test that bootstrap=true is now the default.
#[test]
fn test_default_config_has_bootstrap_enabled() {
    use domain_check_lib::CheckConfig;

    let config = CheckConfig::default();
    assert!(
        config.enable_bootstrap,
        "Bootstrap should be enabled by default"
    );
}

/// Test new exports are accessible from the public API.
#[test]
fn test_new_exports_accessible() {
    // These should compile — they're the new public API functions
    let _ = domain_check_lib::initialize_bootstrap;
    let _ = domain_check_lib::get_whois_server;
}
