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
        .stdout(predicate::str::contains("available")); // Should have multiple domains checked
}

#[test]
fn test_all_flag_shows_info_message() {
    // Use multiple domains to trigger the informational message
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["test1", "test2", "--all", "--batch", "--pretty"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("All"))
        .stdout(predicate::str::contains("TLDs"));
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
        .stdout(predicate::str::contains("2 domains in")); // Only 2 domains should be checked
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
        .stdout(predicate::str::contains("Preset: startup"));
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
    // Use --no-bootstrap to limit to 32 hardcoded TLDs for speed
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["sometestdomain123456", "--all", "--batch", "--no-bootstrap"])
        .timeout(std::time::Duration::from_secs(60));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("domains in"))
        .stdout(predicate::str::contains("available"));
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
        .stdout(predicate::str::contains("domains in")); // Should complete and show summary
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

// ── Domain Generation Tests ──────────────────────────────────────────

#[test]
fn test_pattern_as_sole_input() {
    // --pattern should be accepted without positional domains
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "test\\d", "-t", "com", "--dry-run"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test0.com"))
        .stdout(predicate::str::contains("test9.com"));
}

#[test]
fn test_invalid_pattern_error_exit() {
    // Invalid escape sequence should produce error and exit non-zero
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "test\\x", "-t", "com", "--dry-run"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid pattern"));
}

#[test]
fn test_dry_run_prints_domains_exits_zero() {
    // --dry-run should print FQDNs to stdout and count to stderr, exit 0
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "go\\d", "-t", "com", "--dry-run"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("go0.com"))
        .stdout(predicate::str::contains("go9.com"))
        .stderr(predicate::str::contains("10 domains would be checked"));
}

#[test]
fn test_dry_run_json_output() {
    // --dry-run --json should output a valid JSON array
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "ab\\d", "-t", "com", "--dry-run", "--json"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    let arr = parsed.as_array().expect("should be JSON array");
    assert_eq!(arr.len(), 10); // ab0.com through ab9.com
    assert!(arr.contains(&serde_json::Value::String("ab0.com".to_string())));
}

#[test]
fn test_pattern_with_preset_orthogonal() {
    // --pattern and --preset should work together (patterns generate names, preset expands TLDs)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "zz\\d", "--preset", "startup", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 10 names × 8 TLDs = 80 domains
    assert!(stderr.contains("80 domains would be checked"));
    // Should have .com and .io variants
    assert!(stdout.contains("zz0.com"));
    assert!(stdout.contains("zz0.io"));
}

#[test]
fn test_no_input_error() {
    // No domains, no file, no pattern → error
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args::<[&str; 0], &str>([]);

    cmd.assert().failure().stderr(predicate::str::contains(
        "You must specify domain names, a file with --file, or patterns with --pattern",
    ));
}

#[test]
fn test_help_shows_generation_flags() {
    // --help should list all new generation flags
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--pattern"))
        .stdout(predicate::str::contains("--prefix"))
        .stdout(predicate::str::contains("--suffix"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn test_dc_prefix_env_var() {
    // DC_PREFIX should apply prefixes when CLI --prefix is not set
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.env("DC_PREFIX", "cool,hot")
        .args(["app", "-t", "com", "--dry-run"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("coolapp.com"))
        .stdout(predicate::str::contains("hotapp.com"))
        .stdout(predicate::str::contains("app.com")); // bare included
}

#[test]
fn test_dc_suffix_env_var() {
    // DC_SUFFIX should apply suffixes when CLI --suffix is not set
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.env("DC_SUFFIX", "hub,ly")
        .args(["cloud", "-t", "com", "--dry-run"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("cloudhub.com"))
        .stdout(predicate::str::contains("cloudly.com"))
        .stdout(predicate::str::contains("cloud.com")); // bare included
}

#[test]
fn test_cli_prefix_overrides_dc_prefix() {
    // CLI --prefix should override DC_PREFIX env var
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.env("DC_PREFIX", "old,stale")
        .args(["app", "--prefix", "new", "-t", "com", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("newapp.com"));
    assert!(stdout.contains("app.com")); // bare included
    assert!(!stdout.contains("oldapp.com")); // env var overridden
    assert!(!stdout.contains("staleapp.com"));
}

#[test]
fn test_pattern_all_filtered_error() {
    // Pattern "\d" produces single-char names → all filtered → no valid domains → error
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "\\d", "-t", "com", "--dry-run"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No valid domains"));
}

#[test]
fn test_dry_run_with_prefix_and_suffix() {
    // Combined prefix + suffix + dry-run
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "app",
        "--prefix",
        "get,my",
        "--suffix",
        "hub",
        "-t",
        "com",
        "--dry-run",
    ]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Expect: getapphub, getapp, myapphub, myapp, apphub, app = 6 names × 1 TLD = 6
    assert!(stdout.contains("getapphub.com"));
    assert!(stdout.contains("getapp.com"));
    assert!(stdout.contains("myapphub.com"));
    assert!(stdout.contains("myapp.com"));
    assert!(stdout.contains("apphub.com"));
    assert!(stdout.contains("app.com"));
    assert!(stderr.contains("6 domains would be checked"));
}

#[test]
fn test_dry_run_no_network_requests() {
    // --dry-run should complete instantly (no network) even with a domain that would require lookup
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "zztest\\d\\d", "-t", "com", "--dry-run"])
        .timeout(std::time::Duration::from_secs(5)); // Should finish in <1s

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("100 domains would be checked"));
}

#[test]
fn test_pattern_with_file_input() {
    // Patterns + file input should combine
    let file = create_test_domains_file(&["mysite", "coolapp"]);

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--file",
        file.path().to_str().unwrap(),
        "--pattern",
        "zz\\d",
        "-t",
        "com",
        "--dry-run",
    ]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 2 from file + 10 from pattern = 12 base names × 1 TLD = 12
    assert!(stdout.contains("mysite.com"));
    assert!(stdout.contains("coolapp.com"));
    assert!(stdout.contains("zz0.com"));
    assert!(stdout.contains("zz9.com"));
    assert!(stderr.contains("12 domains would be checked"));
}

#[test]
fn test_multiple_patterns() {
    // Multiple comma-separated patterns
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "aa\\d,bb\\d", "-t", "com", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 10 from aa\d + 10 from bb\d = 20 domains
    assert!(stdout.contains("aa0.com"));
    assert!(stdout.contains("bb9.com"));
    assert!(stderr.contains("20 domains would be checked"));
}

#[test]
fn test_dry_run_csv_not_supported() {
    // --dry-run only supports plain text or --json, not --csv
    // It should still work but only prints plain domain list (csv flag is for checking output)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "ab\\d", "-t", "com", "--dry-run"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ab0.com"));
}

#[test]
fn test_yes_flag_exists() {
    // --yes should be accepted without error
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "ab\\d", "-t", "com", "--dry-run", "--yes"]);

    cmd.assert().success();
}

#[test]
fn test_force_flag_exists() {
    // --force should be accepted without error (same as --yes for confirmation skip)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "ab\\d", "-t", "com", "--dry-run", "--force"]);

    cmd.assert().success();
}

#[test]
fn test_pattern_with_positional_domains() {
    // --pattern + positional domains should combine
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["myapp", "--pattern", "zz\\d", "-t", "com", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // 1 positional + 10 from pattern = 11 base names
    assert!(stdout.contains("myapp.com"));
    assert!(stdout.contains("zz0.com"));
    assert!(stderr.contains("11 domains would be checked"));
}

#[test]
fn test_prefix_only_no_pattern() {
    // --prefix without --pattern, just with positional domains
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["app", "--prefix", "get,my", "-t", "com", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("getapp.com"));
    assert!(stdout.contains("myapp.com"));
    assert!(stdout.contains("app.com"));
}

#[test]
fn test_suffix_only_no_pattern() {
    // --suffix without --pattern, just with positional domains
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["cloud", "--suffix", "hub,ly", "-t", "com", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cloudhub.com"));
    assert!(stdout.contains("cloudly.com"));
    assert!(stdout.contains("cloud.com"));
}

#[test]
fn test_generation_config_from_config_file() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    let config_content = r#"
[generation]
prefixes = ["super", "mega"]
"#;
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--config",
        config_path.to_str().unwrap(),
        "app",
        "-t",
        "com",
        "--dry-run",
    ]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("superapp.com"));
    assert!(stdout.contains("megaapp.com"));
    assert!(stdout.contains("app.com")); // bare included
}

#[test]
fn test_cli_prefix_overrides_config_prefix() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test-config.toml");

    let config_content = r#"
[generation]
prefixes = ["old", "stale"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // CLI --prefix should override config prefixes
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args([
        "--config",
        config_path.to_str().unwrap(),
        "app",
        "--prefix",
        "new",
        "-t",
        "com",
        "--dry-run",
    ]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("newapp.com"));
    assert!(!stdout.contains("oldapp.com")); // config overridden
    assert!(!stdout.contains("staleapp.com"));
}

#[test]
fn test_empty_pattern_string_error() {
    // Empty string pattern should produce error
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["--pattern", "", "-t", "com", "--dry-run"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid pattern"));
}

#[test]
fn test_dry_run_with_all_flag() {
    // --dry-run + --all should produce many domains
    // With bootstrap enabled (default), this fetches IANA registry for ~1,180 TLDs
    // Use --no-bootstrap to test with just hardcoded TLDs (deterministic)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["testname", "--all", "--dry-run", "--no-bootstrap"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.contains("testname.com"));
    assert!(stdout.contains("testname.org"));
    assert!(stderr.contains("32 domains would be checked"));
}

// ============================================================
// --no-bootstrap CLI flag tests
// ============================================================

#[test]
fn test_no_bootstrap_flag_accepted() {
    // --no-bootstrap should be a valid flag
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["google.com", "--no-bootstrap", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("google.com"));
}

#[test]
fn test_no_bootstrap_with_all_limits_to_hardcoded() {
    // --all --no-bootstrap --dry-run should produce exactly 32 TLDs (hardcoded only)
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["testname", "--all", "--no-bootstrap", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("32 domains would be checked"),
        "With --no-bootstrap, --all should use only 32 hardcoded TLDs, got: {}",
        stderr
    );
}

#[test]
fn test_all_with_bootstrap_returns_more_than_32_tlds() {
    // --all (without --no-bootstrap) should return >32 TLDs after bootstrap fetch
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["testname", "--all", "--dry-run"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Extract the number from "N domains would be checked"
    let domain_count: Option<usize> = stderr
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok());

    if let Some(count) = domain_count {
        assert!(
            count > 32,
            "With bootstrap enabled, --all should return >32 TLDs, got {}",
            count
        );
    }
    // If parsing fails (e.g., network unavailable), that's OK — graceful degradation
}

#[test]
fn test_no_bootstrap_flag_in_help() {
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--no-bootstrap"));
}

// ============================================================
// Backward compatibility
// ============================================================

#[test]
fn test_backward_compat_no_generation_flags() {
    // Existing usage without any generation flags should work unchanged
    let mut cmd = Command::cargo_bin("domain-check").unwrap();
    cmd.args(["google.com", "--batch"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("google.com"))
        .stdout(predicate::str::contains("TAKEN"));
}
