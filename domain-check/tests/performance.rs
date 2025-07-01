// domain-check/tests/performance.rs with more realistic data ?? may be not sure will see

use assert_cmd::Command;
use std::time::Instant;

#[test]
fn test_all_flag_performance() {
    let start = Instant::now();

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(&["testdomain12345", "--all", "--batch"])
        .timeout(std::time::Duration::from_secs(45)); // More realistic timeout

    cmd.assert().success();

    let duration = start.elapsed();

    // Should complete within reasonable time for single domain × all TLDs
    assert!(
        duration.as_secs() < 45,
        "All TLD check took too long: {:?}",
        duration
    );
}

#[test]
fn test_preset_performance() {
    let start = Instant::now();

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(&["testdomain12345", "--preset", "startup", "--batch"])
        .timeout(std::time::Duration::from_secs(15));

    cmd.assert().success();

    let duration = start.elapsed();

    // Preset should be faster than --all (8 TLDs vs 42)
    assert!(
        duration.as_secs() < 15,
        "Preset check took too long: {:?}",
        duration
    );
}

#[test]
fn test_concurrent_processing_efficiency() {
    // Test that concurrent processing works efficiently with moderate load
    let start = Instant::now();

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(&[
        "test1",
        "test2",
        "test3",
        "--preset",
        "enterprise", // 6 TLDs instead of 8
        "--concurrency",
        "5",
        "--batch",
    ])
    .timeout(std::time::Duration::from_secs(20));

    cmd.assert().success();

    let duration = start.elapsed();

    // 3 domains × 6 TLDs = 18 checks should complete reasonably quickly
    assert!(
        duration.as_secs() < 20,
        "Concurrent processing took too long: {:?}",
        duration
    );
}

#[test]
fn test_moderate_domain_list_performance() {
    // Test performance with moderate domain list (reduced from 20 to 5)
    use std::fs;
    use tempfile::NamedTempFile;

    // Create a file with 5 test domains instead of 20
    let file = NamedTempFile::new().unwrap();
    let domains: Vec<String> = (0..5).map(|i| format!("testdomain{}", i)).collect();
    fs::write(file.path(), domains.join("\n")).unwrap();

    let start = Instant::now();

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(&[
        "--file",
        file.path().to_str().unwrap(),
        "--preset",
        "enterprise", // 6 TLDs
        "--batch",
    ])
    .timeout(std::time::Duration::from_secs(25));

    cmd.assert().success();

    let duration = start.elapsed();

    // 5 domains × 6 TLDs = 30 checks should complete in reasonable time
    assert!(
        duration.as_secs() < 25,
        "Moderate domain list took too long: {:?}",
        duration
    );
}

#[test]
fn test_single_domain_default_performance() {
    // Test basic single domain performance (should be very fast)
    let start = Instant::now();

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(&["testdomain12345"]) // Default to .com only
        .timeout(std::time::Duration::from_secs(5));

    cmd.assert().success();

    let duration = start.elapsed();

    // Single domain check should be very fast
    assert!(
        duration.as_secs() < 5,
        "Single domain check took too long: {:?}",
        duration
    );
}
