// domain-check-lib/tests/integration.rs

//! Integration tests for domain-check-lib new exports

use domain_check_lib::{get_all_known_tlds, get_available_presets, get_preset_tlds};

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
    assert_eq!(presets.len(), 3);
    assert!(presets.contains(&"startup"));
    assert!(presets.contains(&"enterprise"));
    assert!(presets.contains(&"country"));
}

#[test]
fn test_preset_tlds_are_subset_of_all_tlds() {
    let all_tlds = get_all_known_tlds();
    let presets = get_available_presets();

    for preset_name in presets {
        let preset_tlds = get_preset_tlds(preset_name).unwrap();
        for tld in preset_tlds {
            assert!(
                all_tlds.contains(&tld),
                "Preset '{}' contains TLD '{}' not in all_known_tlds",
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
}

#[test]
fn test_preset_tlds_invalid_returns_none() {
    assert!(get_preset_tlds("nonexistent").is_none());
    assert!(get_preset_tlds("").is_none());
    assert!(get_preset_tlds("invalid_preset_name").is_none());
}
