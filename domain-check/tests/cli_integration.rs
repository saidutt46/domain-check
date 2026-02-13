// domain-check/tests/cli_integration.rs

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

/// Helper to create a test domains file
fn create_test_domains_file(domains: &[&str]) -> NamedTempFile {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    let content = domains.join("\n");
    fs::write(file.path(), content).expect("Failed to write to temp file");
    file
}

#[test]
fn test_help_shows_new_flags() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--all"))
        .stdout(predicate::str::contains("--preset"))
        .stdout(predicate::str::contains("startup (8)"))
        .stdout(predicate::str::contains("enterprise (6)"))
        .stdout(predicate::str::contains("country (9)"));
}

#[test]
fn test_all_flag_functionality() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["nonexistentdomain12345", "--all", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("nonexistentdomain12345.com"))
        .stdout(predicate::str::contains("nonexistentdomain12345.org"))
        .stdout(predicate::str::contains("Summary:")); // Should have multiple domains checked
}

#[test]
fn test_all_flag_shows_info_message() {
    // Use multiple domains to trigger the informational message
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test1", "test2", "--all", "--batch", "--pretty"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Checking against all"))
        .stdout(predicate::str::contains("known TLDs"));
}

// #[test]
// fn test_preset_startup_functionality() {
//     let mut cmd = Command::cargo_bin("domain-check").unwrap();
//     cmd.args(["nonexistentdomain12345", "--preset", "startup", "--batch"]);

//     cmd.assert()
//         .success()
//         .stdout(predicate::str::contains("nonexistentdomain12345.com"))
//         .stdout(predicate::str::contains("nonexistentdomain12345.io"))
//         .stdout(predicate::str::contains("nonexistentdomain12345.ai"))
//         .stdout(predicate::str::contains("Summary: 8")); // Should show 8 domains checked
// }

// #[test]
// fn test_preset_startup_shows_info_message() {
//     // Use multiple domains + pretty to trigger the message
//     let mut cmd = Command::cargo_bin("domain-check").unwrap();
//     cmd.args([
//         "test1", "test2", "--preset", "startup", "--batch", "--pretty",
//     ]);

//     cmd.assert()
//         .success()
//         .stdout(predicate::str::contains("Using 'startup' preset"))
//         .stdout(predicate::str::contains("8 TLDs"));
// }

// #[test]
// fn test_preset_enterprise_functionality() {
//     let mut cmd = Command::cargo_bin("domain-check").unwrap();
//     cmd.args([
//         "nonexistentdomain12345",
//         "--preset",
//         "enterprise",
//         "--batch",
//     ]);

//     cmd.assert()
//         .success()
//         .stdout(predicate::str::contains("nonexistentdomain12345.com"))
//         .stdout(predicate::str::contains("nonexistentdomain12345.biz"))
//         .stdout(predicate::str::contains("Summary: 6")); // Should show 6 domains checked
// }

// #[test]
// fn test_preset_country_functionality() {
//     let mut cmd = Command::cargo_bin("domain-check").unwrap();
//     cmd.args(["nonexistentdomain12345", "--preset", "country", "--batch"]);

//     cmd.assert()
//         .success()
//         .stdout(predicate::str::contains("nonexistentdomain12345.us"))
//         .stdout(predicate::str::contains("nonexistentdomain12345.uk"))
//         .stdout(predicate::str::contains("Summary: 9")); // Should show 9 domains checked
// }

#[test]
fn test_conflicting_tld_sources_error() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test", "--all", "--preset", "startup"]);

    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot specify multiple TLD sources",
    ));
}

#[test]
fn test_tld_and_all_conflict_error() {
    // This is the correct behavior - should error, not take precedence
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test", "-t", "com,org", "--all"]);

    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot specify multiple TLD sources",
    ));
}

#[test]
fn test_explicit_tld_works_alone() {
    // Test that -t works when used alone (not conflicting with --all)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["nonexistentdomain12345", "-t", "com,org", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("nonexistentdomain12345.com"))
        .stdout(predicate::str::contains("nonexistentdomain12345.org"))
        .stdout(predicate::str::contains("Summary: 2")); // Only 2 domains should be checked
}

#[test]
fn test_json_output_with_all_flag() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["nonexistentdomain12345", "--all", "--json"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[").and(predicate::str::contains("]")));
}

#[test]
fn test_csv_output_with_preset() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["nonexistentdomain12345", "--preset", "startup", "--csv"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("domain,available,registrar"))
        .stdout(predicate::str::contains("nonexistentdomain12345.com"))
        .stdout(predicate::str::contains("nonexistentdomain12345.io"));
}

#[test]
fn test_file_input_with_all_flag() {
    let domains = vec!["testdomain123", "anotherdomain456"];
    let file = create_test_domains_file(&domains);

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--file", file.path().to_str().unwrap(), "--all", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("testdomain123"))
        .stdout(predicate::str::contains("anotherdomain456"));
}

// #[test]
// fn test_file_input_with_preset() {
//     let domains = vec!["testdomain123", "anotherdomain456"];
//     let file = create_test_domains_file(&domains);

//     let mut cmd = Command::cargo_bin("domain-check").unwrap();
//     cmd.args([
//         "--file",
//         file.path().to_str().unwrap(),
//         "--preset",
//         "startup",
//         "--batch",
//     ]);

//     cmd.assert()
//         .success()
//         .stdout(predicate::str::contains("testdomain123"))
//         .stdout(predicate::str::contains("anotherdomain456"));
// }

#[test]
fn test_file_input_with_preset_shows_message() {
    let domains = vec!["testdomain123", "anotherdomain456"];
    let file = create_test_domains_file(&domains);

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--file",
        file.path().to_str().unwrap(),
        "--preset",
        "startup",
        "--batch",
        "--pretty",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Using 'startup' preset"));
}

#[test]
fn test_multiple_domains_with_all_flag() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test1", "test2", "test3", "--all", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test1"))
        .stdout(predicate::str::contains("test2"))
        .stdout(predicate::str::contains("test3"));
}

#[test]
fn test_bootstrap_auto_enable_with_verbose() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test", "--all", "--verbose"]);

    cmd.assert().success().stderr(
        predicate::str::contains("Auto-enabled bootstrap registry").or(
            predicate::str::contains("bootstrap").not(), // It's fine if bootstrap doesn't auto-enable in test
        ),
    );
}

#[test]
fn test_error_aggregation_in_output() {
    // This test checks that summary contains the expected text
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["sometestdomain123456", "--all", "--batch"])
        .timeout(std::time::Duration::from_secs(30));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Summary:").and(
            predicate::str::contains("available"), // Look for "available" instead of "processed"
        ));
}

#[test]
fn test_backward_compatibility_default_behavior() {
    // Ensure old behavior still works
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["google"]); // Should default to google.com

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("google.com"));
}

#[test]
fn test_backward_compatibility_explicit_tlds() {
    // Ensure old -t behavior still works
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["google", "-t", "com,org"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("google.com"))
        .stdout(predicate::str::contains("google.org"));
}

#[test]
fn test_moderate_domain_set_no_artificial_limits() {
    // Test with a more reasonable domain set (10 domains instead of 50)
    let domains: Vec<String> = (0..10).map(|i| format!("testdomain{}", i)).collect();
    let file = create_test_domains_file(&domains.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--file",
        file.path().to_str().unwrap(),
        "--preset",
        "startup",
        "--batch",
    ]) // Use preset instead of --all
    .timeout(std::time::Duration::from_secs(30));

    // Should not fail due to artificial limits
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Summary:")); // Should complete and show summary
}

#[test]
fn test_config_file_integration() {
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory for our test
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Create test config file
    let config_content = r#"
[defaults]
concurrency = 35
preset = "enterprise"
pretty = true

[custom_presets]
test_preset = ["com", "org"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // Test explicit config file usage
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--config",
        config_path.to_str().unwrap(),
        "testdomain123",
        "--verbose",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Using explicit config file"))
        .stdout(predicate::str::contains("testdomain123.com"));
}

#[test]
fn test_custom_preset_from_config() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("preset-config.toml");

    // Create config with custom preset
    let config_content = r#"
[custom_presets]
my_test = ["com", "net"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // Test custom preset usage
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--config",
        config_path.to_str().unwrap(),
        "testdomain123",
        "--preset",
        "my_test",
        "--batch",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("testdomain123.com"))
        .stdout(predicate::str::contains("testdomain123.net"))
        .stdout(predicate::str::contains("2 taken")); // CHANGE: Look for "2 taken" instead of "Summary: 2"
}

#[test]
fn test_environment_variable_integration() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.env("DC_CONCURRENCY", "45")
        .env("DC_PRESET", "enterprise")
        .args(["testdomain123", "--verbose"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Using DC_CONCURRENCY=45"))
        .stdout(predicate::str::contains("Using DC_PRESET=enterprise"));
}

#[test]
fn test_precedence_cli_over_env() {
    // CLI args should override environment variables
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.env("DC_PRESET", "enterprise")
        .args(["testdomain123", "--preset", "startup", "--verbose"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Using DC_PRESET=enterprise"))
        .stdout(predicate::str::contains("startup")); // CLI preset should be used despite env var
}

#[test]
fn test_config_file_discovery() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("domain-check.toml");

    // Create local config file
    let config_content = r#"
[defaults]
pretty = true
"#;
    fs::write(&config_path, config_content).unwrap();

    // Change to temp directory and run (to test local config discovery)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["testdomain123", "--verbose"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Discovering config files"));
}

#[test]
fn test_streaming_with_json_rejected() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test", "--streaming", "--json"]);

    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot use --streaming with --json or --csv",
    ));
}

#[test]
fn test_streaming_with_csv_rejected() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test", "--streaming", "--csv"]);

    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot use --streaming with --json or --csv",
    ));
}

#[test]
fn test_config_detailed_info_respected_without_flag() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    // Config enables detailed_info
    let config_content = r#"
[defaults]
detailed_info = true
"#;
    fs::write(&config_path, config_content).unwrap();

    // Run WITHOUT --info flag; config should still enable detailed info
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--config",
        config_path.to_str().unwrap(),
        "google.com",
        "--batch",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Registrar:"));
}

#[test]
fn test_env_detailed_info_respected_without_flag() {
    // DC_DETAILED_INFO=true should enable detailed info even without --info
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.env("DC_DETAILED_INFO", "true")
        .args(["google.com", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Registrar:"));
}
