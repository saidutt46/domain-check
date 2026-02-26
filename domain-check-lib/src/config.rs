//! Configuration file parsing and management.
//!
//! This module handles loading configuration from TOML files and merging
//! configurations with proper precedence rules.

use crate::error::DomainCheckError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration loaded from TOML files.
///
/// This represents the structure of configuration files that users can create
/// to set default values and custom presets.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileConfig {
    /// Default values for CLI options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<DefaultsConfig>,

    /// User-defined TLD presets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_presets: Option<HashMap<String, Vec<String>>>,

    /// Monitoring configuration (future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<MonitoringConfig>,

    /// Output formatting preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputConfig>,

    /// Domain generation defaults (prefixes/suffixes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<GenerationConfig>,
}

/// Default configuration values that map to CLI options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default concurrency level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,

    /// Default TLD preset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,

    /// Default TLD list (alternative to preset)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlds: Option<Vec<String>>,

    /// Default pretty output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretty: Option<bool>,

    /// Default timeout (as string, e.g., "5s", "30s")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Default WHOIS fallback setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whois_fallback: Option<bool>,

    /// Default bootstrap setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<bool>,

    /// Default detailed info setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_info: Option<bool>,
}

/// Monitoring configuration (placeholder for future features).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonitoringConfig {
    /// Monitoring interval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,

    /// Command to run on changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_command: Option<String>,
}

/// Domain generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationConfig {
    /// Default prefixes to prepend to domain names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefixes: Option<Vec<String>>,

    /// Default suffixes to append to domain names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffixes: Option<Vec<String>>,
}

/// Output formatting configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    /// Default output format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_format: Option<String>,

    /// Include CSV headers by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csv_headers: Option<bool>,

    /// Pretty-print JSON by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_pretty: Option<bool>,
}

/// Configuration discovery and loading functionality.
pub struct ConfigManager {
    /// Whether to emit warnings for config issues
    pub verbose: bool,
}

impl ConfigManager {
    /// Create a new configuration manager.
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Load configuration from a specific file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The parsed configuration or an error if parsing fails.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<FileConfig, DomainCheckError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(DomainCheckError::file_error(
                path.to_string_lossy(),
                "Configuration file not found",
            ));
        }

        let content = fs::read_to_string(path).map_err(|e| {
            DomainCheckError::file_error(
                path.to_string_lossy(),
                format!("Failed to read configuration file: {}", e),
            )
        })?;

        let config: FileConfig =
            toml::from_str(&content).map_err(|e| DomainCheckError::ConfigError {
                message: format!("Failed to parse TOML configuration: {}", e),
            })?;

        // Validate the loaded configuration
        self.validate_config(&config)?;

        Ok(config)
    }

    /// Discover and load configuration files in precedence order.
    ///
    /// Looks for configuration files in standard locations and merges them
    /// according to precedence rules.
    ///
    /// # Returns
    ///
    /// Merged configuration from all discovered files.
    pub fn discover_and_load(&self) -> Result<FileConfig, DomainCheckError> {
        let mut merged_config = FileConfig::default();
        let mut loaded_files = Vec::new();

        // 1. Load XDG config (lowest precedence)
        if let Some(xdg_path) = self.get_xdg_config_path() {
            if let Ok(config) = self.load_file(&xdg_path) {
                merged_config = self.merge_configs(merged_config, config);
                loaded_files.push(xdg_path);
            }
        }

        // 2. Load global config
        if let Some(global_path) = self.get_global_config_path() {
            if let Ok(config) = self.load_file(&global_path) {
                merged_config = self.merge_configs(merged_config, config);
                loaded_files.push(global_path);
            }
        }

        // 3. Load local config (highest precedence)
        if let Some(local_path) = self.get_local_config_path() {
            if let Ok(config) = self.load_file(&local_path) {
                merged_config = self.merge_configs(merged_config, config);
                loaded_files.push(local_path);
            }
        }

        // Warn about multiple config files if verbose
        if self.verbose && loaded_files.len() > 1 {
            eprintln!("⚠️  Multiple config files found. Using precedence:");
            for (i, path) in loaded_files.iter().enumerate() {
                let status = if i == loaded_files.len() - 1 {
                    "active"
                } else {
                    "ignored"
                };
                eprintln!("   {} ({})", path.display(), status);
            }
        }

        Ok(merged_config)
    }

    /// Get the local configuration file path.
    ///
    /// Looks for configuration files in the current directory.
    fn get_local_config_path(&self) -> Option<PathBuf> {
        let candidates = ["./domain-check.toml", "./.domain-check.toml"];

        for candidate in &candidates {
            let path = Path::new(candidate);
            if path.exists() {
                return Some(path.to_path_buf());
            }
        }

        None
    }

    /// Get the global configuration file path.
    ///
    /// Looks for configuration files in the user's home directory.
    fn get_global_config_path(&self) -> Option<PathBuf> {
        if let Some(home) = env::var_os("HOME") {
            let candidates = [".domain-check.toml", "domain-check.toml"];

            for candidate in &candidates {
                let path = Path::new(&home).join(candidate);
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Get the XDG configuration file path.
    ///
    /// Follows the XDG Base Directory Specification.
    fn get_xdg_config_path(&self) -> Option<PathBuf> {
        let config_dir = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;

        let path = config_dir.join("domain-check").join("config.toml");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Merge two configurations with proper precedence.
    ///
    /// Values from `higher` take precedence over values from `lower`.
    fn merge_configs(&self, lower: FileConfig, higher: FileConfig) -> FileConfig {
        FileConfig {
            defaults: match (lower.defaults, higher.defaults) {
                (Some(mut lower_defaults), Some(higher_defaults)) => {
                    // Merge defaults with higher precedence winning
                    if higher_defaults.concurrency.is_some() {
                        lower_defaults.concurrency = higher_defaults.concurrency;
                    }
                    if higher_defaults.preset.is_some() {
                        lower_defaults.preset = higher_defaults.preset;
                    }
                    if higher_defaults.tlds.is_some() {
                        lower_defaults.tlds = higher_defaults.tlds;
                    }
                    if higher_defaults.pretty.is_some() {
                        lower_defaults.pretty = higher_defaults.pretty;
                    }
                    if higher_defaults.timeout.is_some() {
                        lower_defaults.timeout = higher_defaults.timeout;
                    }
                    if higher_defaults.whois_fallback.is_some() {
                        lower_defaults.whois_fallback = higher_defaults.whois_fallback;
                    }
                    if higher_defaults.bootstrap.is_some() {
                        lower_defaults.bootstrap = higher_defaults.bootstrap;
                    }
                    if higher_defaults.detailed_info.is_some() {
                        lower_defaults.detailed_info = higher_defaults.detailed_info;
                    }
                    Some(lower_defaults)
                }
                (None, Some(higher_defaults)) => Some(higher_defaults),
                (Some(lower_defaults), None) => Some(lower_defaults),
                (None, None) => None,
            },
            custom_presets: match (lower.custom_presets, higher.custom_presets) {
                (Some(mut lower_presets), Some(higher_presets)) => {
                    // Merge custom presets, higher precedence wins for conflicts
                    lower_presets.extend(higher_presets);
                    Some(lower_presets)
                }
                (None, Some(higher_presets)) => Some(higher_presets),
                (Some(lower_presets), None) => Some(lower_presets),
                (None, None) => None,
            },
            monitoring: higher.monitoring.or(lower.monitoring),
            output: higher.output.or(lower.output),
            generation: match (lower.generation, higher.generation) {
                (Some(mut lower_gen), Some(higher_gen)) => {
                    if higher_gen.prefixes.is_some() {
                        lower_gen.prefixes = higher_gen.prefixes;
                    }
                    if higher_gen.suffixes.is_some() {
                        lower_gen.suffixes = higher_gen.suffixes;
                    }
                    Some(lower_gen)
                }
                (None, Some(higher_gen)) => Some(higher_gen),
                (Some(lower_gen), None) => Some(lower_gen),
                (None, None) => None,
            },
        }
    }

    /// Validate a configuration for common issues.
    fn validate_config(&self, config: &FileConfig) -> Result<(), DomainCheckError> {
        if let Some(defaults) = &config.defaults {
            // Validate concurrency
            if let Some(concurrency) = defaults.concurrency {
                if concurrency == 0 || concurrency > 100 {
                    return Err(DomainCheckError::ConfigError {
                        message: "Concurrency must be between 1 and 100".to_string(),
                    });
                }
            }

            // Validate timeout format
            if let Some(timeout_str) = &defaults.timeout {
                if parse_timeout_string(timeout_str).is_none() {
                    return Err(DomainCheckError::ConfigError {
                        message: format!(
                            "Invalid timeout format '{}'. Use format like '5s', '30s', '2m'",
                            timeout_str
                        ),
                    });
                }
            }

            // Validate that preset and tlds are not both specified
            if defaults.preset.is_some() && defaults.tlds.is_some() {
                return Err(DomainCheckError::ConfigError {
                    message: "Cannot specify both 'preset' and 'tlds' in defaults".to_string(),
                });
            }
        }

        // Validate custom presets
        if let Some(presets) = &config.custom_presets {
            for (name, tlds) in presets {
                if name.is_empty() {
                    return Err(DomainCheckError::ConfigError {
                        message: "Custom preset names cannot be empty".to_string(),
                    });
                }

                if tlds.is_empty() {
                    return Err(DomainCheckError::ConfigError {
                        message: format!("Custom preset '{}' cannot have empty TLD list", name),
                    });
                }

                // Basic TLD format validation
                for tld in tlds {
                    if tld.is_empty() || tld.contains('.') || tld.contains(' ') {
                        return Err(DomainCheckError::ConfigError {
                            message: format!("Invalid TLD '{}' in preset '{}'", tld, name),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

/// Environment variable configuration that mirrors CLI options.
///
/// This represents configuration values that can be set via DC_* environment variables.
#[derive(Debug, Clone, Default)]
pub struct EnvConfig {
    pub concurrency: Option<usize>,
    pub preset: Option<String>,
    pub tlds: Option<Vec<String>>,
    pub pretty: Option<bool>,
    pub timeout: Option<String>,
    pub whois_fallback: Option<bool>,
    pub bootstrap: Option<bool>,
    pub detailed_info: Option<bool>,
    pub json: Option<bool>,
    pub csv: Option<bool>,
    pub file: Option<String>,
    pub config: Option<String>,
    pub prefixes: Option<Vec<String>>,
    pub suffixes: Option<Vec<String>>,
}

/// Load configuration from environment variables.
///
/// Parses all DC_* environment variables and returns a structured configuration.
/// Invalid values are logged as warnings and ignored.
///
/// # Arguments
///
/// * `verbose` - Whether to log environment variable usage
///
/// # Returns
///
/// Parsed environment configuration with validated values.
pub fn load_env_config(verbose: bool) -> EnvConfig {
    let mut env_config = EnvConfig::default();

    // DC_CONCURRENCY - concurrent domain checks
    if let Ok(val) = env::var("DC_CONCURRENCY") {
        match val.parse::<usize>() {
            Ok(concurrency) if concurrency > 0 && concurrency <= 100 => {
                env_config.concurrency = Some(concurrency);
                if verbose {
                    println!("🔧 Using DC_CONCURRENCY={}", concurrency);
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_CONCURRENCY='{}', must be 1-100", val);
                }
            }
        }
    }

    // DC_PRESET - TLD preset name
    if let Ok(preset) = env::var("DC_PRESET") {
        if !preset.trim().is_empty() {
            env_config.preset = Some(preset.clone());
            if verbose {
                println!("🔧 Using DC_PRESET={}", preset);
            }
        }
    }

    // DC_TLD - comma-separated TLD list
    if let Ok(tld_str) = env::var("DC_TLD") {
        let tlds: Vec<String> = tld_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !tlds.is_empty() {
            env_config.tlds = Some(tlds);
            if verbose {
                println!("🔧 Using DC_TLD={}", tld_str);
            }
        }
    }

    // DC_PRETTY - enable pretty output
    if let Ok(val) = env::var("DC_PRETTY") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                env_config.pretty = Some(true);
                if verbose {
                    println!("🔧 Using DC_PRETTY=true");
                }
            }
            "false" | "0" | "no" | "off" => {
                env_config.pretty = Some(false);
                if verbose {
                    println!("🔧 Using DC_PRETTY=false");
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_PRETTY='{}', use true/false", val);
                }
            }
        }
    }

    // DC_TIMEOUT - timeout setting
    if let Ok(timeout_str) = env::var("DC_TIMEOUT") {
        // Validate timeout format
        if parse_timeout_string(&timeout_str).is_some() {
            env_config.timeout = Some(timeout_str.clone());
            if verbose {
                println!("🔧 Using DC_TIMEOUT={}", timeout_str);
            }
        } else if verbose {
            eprintln!(
                "⚠️ Invalid DC_TIMEOUT='{}', use format like '5s', '30s', '2m'",
                timeout_str
            );
        }
    }

    // DC_WHOIS_FALLBACK - enable/disable WHOIS fallback
    if let Ok(val) = env::var("DC_WHOIS_FALLBACK") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                env_config.whois_fallback = Some(true);
                if verbose {
                    println!("🔧 Using DC_WHOIS_FALLBACK=true");
                }
            }
            "false" | "0" | "no" | "off" => {
                env_config.whois_fallback = Some(false);
                if verbose {
                    println!("🔧 Using DC_WHOIS_FALLBACK=false");
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_WHOIS_FALLBACK='{}', use true/false", val);
                }
            }
        }
    }

    // DC_BOOTSTRAP - enable/disable IANA bootstrap
    if let Ok(val) = env::var("DC_BOOTSTRAP") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                env_config.bootstrap = Some(true);
                if verbose {
                    println!("🔧 Using DC_BOOTSTRAP=true");
                }
            }
            "false" | "0" | "no" | "off" => {
                env_config.bootstrap = Some(false);
                if verbose {
                    println!("🔧 Using DC_BOOTSTRAP=false");
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_BOOTSTRAP='{}', use true/false", val);
                }
            }
        }
    }

    // DC_DETAILED_INFO - enable detailed domain info
    if let Ok(val) = env::var("DC_DETAILED_INFO") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                env_config.detailed_info = Some(true);
                if verbose {
                    println!("🔧 Using DC_DETAILED_INFO=true");
                }
            }
            "false" | "0" | "no" | "off" => {
                env_config.detailed_info = Some(false);
                if verbose {
                    println!("🔧 Using DC_DETAILED_INFO=false");
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_DETAILED_INFO='{}', use true/false", val);
                }
            }
        }
    }

    // DC_JSON - enable JSON output
    if let Ok(val) = env::var("DC_JSON") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                env_config.json = Some(true);
                if verbose {
                    println!("🔧 Using DC_JSON=true");
                }
            }
            "false" | "0" | "no" | "off" => {
                env_config.json = Some(false);
                if verbose {
                    println!("🔧 Using DC_JSON=false");
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_JSON='{}', use true/false", val);
                }
            }
        }
    }

    // DC_CSV - enable CSV output
    if let Ok(val) = env::var("DC_CSV") {
        match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                env_config.csv = Some(true);
                if verbose {
                    println!("🔧 Using DC_CSV=true");
                }
            }
            "false" | "0" | "no" | "off" => {
                env_config.csv = Some(false);
                if verbose {
                    println!("🔧 Using DC_CSV=false");
                }
            }
            _ => {
                if verbose {
                    eprintln!("⚠️ Invalid DC_CSV='{}', use true/false", val);
                }
            }
        }
    }

    // DC_FILE - default domains file
    if let Ok(file_path) = env::var("DC_FILE") {
        if !file_path.trim().is_empty() {
            env_config.file = Some(file_path.clone());
            if verbose {
                println!("🔧 Using DC_FILE={}", file_path);
            }
        }
    }

    // DC_CONFIG - default config file
    if let Ok(config_path) = env::var("DC_CONFIG") {
        if !config_path.trim().is_empty() {
            env_config.config = Some(config_path.clone());
            if verbose {
                println!("🔧 Using DC_CONFIG={}", config_path);
            }
        }
    }

    // DC_PREFIX - comma-separated prefixes for domain generation
    if let Ok(prefix_str) = env::var("DC_PREFIX") {
        let prefixes: Vec<String> = prefix_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !prefixes.is_empty() {
            env_config.prefixes = Some(prefixes);
            if verbose {
                println!("🔧 Using DC_PREFIX={}", prefix_str);
            }
        }
    }

    // DC_SUFFIX - comma-separated suffixes for domain generation
    if let Ok(suffix_str) = env::var("DC_SUFFIX") {
        let suffixes: Vec<String> = suffix_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !suffixes.is_empty() {
            env_config.suffixes = Some(suffixes);
            if verbose {
                println!("🔧 Using DC_SUFFIX={}", suffix_str);
            }
        }
    }

    env_config
}

/// Convert EnvConfig to equivalent CLI arguments format for precedence handling.
///
/// This allows environment variables to be processed using the same logic as CLI args.
impl EnvConfig {
    /// Get the preset value, checking for conflicts with explicit TLD list.
    pub fn get_effective_preset(&self) -> Option<String> {
        // If explicit TLDs are set, preset is ignored
        if self.tlds.is_some() {
            None
        } else {
            self.preset.clone()
        }
    }

    /// Get the effective TLD list, preferring explicit TLDs over preset.
    pub fn get_effective_tlds(&self) -> Option<Vec<String>> {
        self.tlds.clone()
    }

    /// Check if output format conflicts exist (JSON and CSV both set).
    pub fn has_output_format_conflict(&self) -> bool {
        matches!((self.json, self.csv), (Some(true), Some(true)))
    }
}

/// Parse a timeout string like "5s", "30s", "2m" into seconds.
///
/// # Arguments
///
/// * `timeout_str` - String representation of timeout
///
/// # Returns
///
/// Number of seconds, or None if parsing fails.
fn parse_timeout_string(timeout_str: &str) -> Option<u64> {
    let timeout_str = timeout_str.trim().to_lowercase();

    if timeout_str.ends_with('s') {
        timeout_str
            .strip_suffix('s')
            .and_then(|s| s.parse::<u64>().ok())
    } else if timeout_str.ends_with('m') {
        timeout_str
            .strip_suffix('m')
            .and_then(|s| s.parse::<u64>().ok())
            .map(|m| m * 60)
    } else {
        // Assume seconds if no unit
        timeout_str.parse::<u64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ── Helper ──────────────────────────────────────────────────────────

    fn write_temp_config(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    // ── parse_timeout_string ────────────────────────────────────────────

    #[test]
    fn test_parse_timeout_seconds_with_suffix() {
        assert_eq!(parse_timeout_string("5s"), Some(5));
        assert_eq!(parse_timeout_string("30s"), Some(30));
        assert_eq!(parse_timeout_string("0s"), Some(0));
        assert_eq!(parse_timeout_string("999s"), Some(999));
    }

    #[test]
    fn test_parse_timeout_minutes() {
        assert_eq!(parse_timeout_string("2m"), Some(120));
        assert_eq!(parse_timeout_string("1m"), Some(60));
        assert_eq!(parse_timeout_string("0m"), Some(0));
    }

    #[test]
    fn test_parse_timeout_bare_number() {
        assert_eq!(parse_timeout_string("5"), Some(5));
        assert_eq!(parse_timeout_string("0"), Some(0));
        assert_eq!(parse_timeout_string("120"), Some(120));
    }

    #[test]
    fn test_parse_timeout_whitespace_trimmed() {
        assert_eq!(parse_timeout_string("  5s  "), Some(5));
        assert_eq!(parse_timeout_string(" 2m "), Some(120));
    }

    #[test]
    fn test_parse_timeout_case_insensitive() {
        assert_eq!(parse_timeout_string("5S"), Some(5));
        assert_eq!(parse_timeout_string("2M"), Some(120));
    }

    #[test]
    fn test_parse_timeout_invalid() {
        assert_eq!(parse_timeout_string("invalid"), None);
        assert_eq!(parse_timeout_string("abc"), None);
        assert_eq!(parse_timeout_string("s"), None);
        assert_eq!(parse_timeout_string("m"), None);
        assert_eq!(parse_timeout_string(""), None);
        assert_eq!(parse_timeout_string("-5s"), None);
    }

    // ── FileConfig defaults ─────────────────────────────────────────────

    #[test]
    fn test_file_config_default_all_none() {
        let config = FileConfig::default();
        assert!(config.defaults.is_none());
        assert!(config.custom_presets.is_none());
        assert!(config.monitoring.is_none());
        assert!(config.output.is_none());
        assert!(config.generation.is_none());
    }

    #[test]
    fn test_defaults_config_default_all_none() {
        let defaults = DefaultsConfig::default();
        assert!(defaults.concurrency.is_none());
        assert!(defaults.preset.is_none());
        assert!(defaults.tlds.is_none());
        assert!(defaults.pretty.is_none());
        assert!(defaults.timeout.is_none());
        assert!(defaults.whois_fallback.is_none());
        assert!(defaults.bootstrap.is_none());
        assert!(defaults.detailed_info.is_none());
    }

    // ── ConfigManager::load_file ────────────────────────────────────────

    #[test]
    fn test_load_valid_config_full() {
        let f = write_temp_config(
            r#"
[defaults]
concurrency = 25
preset = "startup"
pretty = true
timeout = "10s"
whois_fallback = false
bootstrap = true
detailed_info = true

[custom_presets]
my_preset = ["com", "org", "io"]
"#,
        );

        let manager = ConfigManager::new(false);
        let config = manager.load_file(f.path()).unwrap();

        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.concurrency, Some(25));
        assert_eq!(defaults.preset, Some("startup".to_string()));
        assert_eq!(defaults.pretty, Some(true));
        assert_eq!(defaults.timeout, Some("10s".to_string()));
        assert_eq!(defaults.whois_fallback, Some(false));
        assert_eq!(defaults.bootstrap, Some(true));
        assert_eq!(defaults.detailed_info, Some(true));

        let presets = config.custom_presets.unwrap();
        assert_eq!(
            presets.get("my_preset"),
            Some(&vec!["com".into(), "org".into(), "io".into()])
        );
    }

    #[test]
    fn test_load_empty_config() {
        let f = write_temp_config("");
        let manager = ConfigManager::new(false);
        let config = manager.load_file(f.path()).unwrap();
        assert!(config.defaults.is_none());
        assert!(config.custom_presets.is_none());
    }

    #[test]
    fn test_load_minimal_defaults_only() {
        let f = write_temp_config(
            r#"
[defaults]
concurrency = 50
"#,
        );
        let manager = ConfigManager::new(false);
        let config = manager.load_file(f.path()).unwrap();
        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.concurrency, Some(50));
        assert!(defaults.preset.is_none());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let manager = ConfigManager::new(false);
        let result = manager.load_file("/tmp/nonexistent_domain_check_config_xyz.toml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_load_invalid_toml() {
        let f = write_temp_config("this is not [valid toml ===");
        let manager = ConfigManager::new(false);
        let result = manager.load_file(f.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("TOML"));
    }

    // ── Validation: concurrency ─────────────────────────────────────────

    #[test]
    fn test_validate_concurrency_zero() {
        let f = write_temp_config("[defaults]\nconcurrency = 0\n");
        let manager = ConfigManager::new(false);
        let result = manager.load_file(f.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("between 1 and 100"));
    }

    #[test]
    fn test_validate_concurrency_over_100() {
        let f = write_temp_config("[defaults]\nconcurrency = 101\n");
        let manager = ConfigManager::new(false);
        let result = manager.load_file(f.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("between 1 and 100"));
    }

    #[test]
    fn test_validate_concurrency_boundary_1() {
        let f = write_temp_config("[defaults]\nconcurrency = 1\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    #[test]
    fn test_validate_concurrency_boundary_100() {
        let f = write_temp_config("[defaults]\nconcurrency = 100\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    // ── Validation: timeout ─────────────────────────────────────────────

    #[test]
    fn test_validate_timeout_invalid_format() {
        let f = write_temp_config("[defaults]\ntimeout = \"abc\"\n");
        let manager = ConfigManager::new(false);
        let result = manager.load_file(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid timeout"));
    }

    #[test]
    fn test_validate_timeout_valid_seconds() {
        let f = write_temp_config("[defaults]\ntimeout = \"30s\"\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    #[test]
    fn test_validate_timeout_valid_minutes() {
        let f = write_temp_config("[defaults]\ntimeout = \"2m\"\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    #[test]
    fn test_validate_timeout_bare_number_valid() {
        let f = write_temp_config("[defaults]\ntimeout = \"10\"\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    // ── Validation: preset + tlds conflict ──────────────────────────────

    #[test]
    fn test_validate_preset_and_tlds_conflict() {
        let f = write_temp_config(
            r#"
[defaults]
preset = "startup"
tlds = ["com", "org"]
"#,
        );
        let manager = ConfigManager::new(false);
        let result = manager.load_file(f.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both"));
    }

    #[test]
    fn test_validate_preset_alone_ok() {
        let f = write_temp_config("[defaults]\npreset = \"startup\"\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    #[test]
    fn test_validate_tlds_alone_ok() {
        let f = write_temp_config("[defaults]\ntlds = [\"com\", \"org\"]\n");
        let manager = ConfigManager::new(false);
        assert!(manager.load_file(f.path()).is_ok());
    }

    // ── Validation: custom presets ──────────────────────────────────────

    #[test]
    fn test_validate_custom_preset_empty_name() {
        let manager = ConfigManager::new(false);
        let config = FileConfig {
            custom_presets: Some(HashMap::from([("".to_string(), vec!["com".to_string()])])),
            ..Default::default()
        };
        let result = manager.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_custom_preset_empty_tld_list() {
        let manager = ConfigManager::new(false);
        let config = FileConfig {
            custom_presets: Some(HashMap::from([("mypreset".to_string(), vec![])])),
            ..Default::default()
        };
        let result = manager.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty TLD list"));
    }

    #[test]
    fn test_validate_custom_preset_invalid_tld_with_dot() {
        let manager = ConfigManager::new(false);
        let config = FileConfig {
            custom_presets: Some(HashMap::from([(
                "bad".to_string(),
                vec!["co.uk".to_string()],
            )])),
            ..Default::default()
        };
        let result = manager.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid TLD"));
    }

    #[test]
    fn test_validate_custom_preset_invalid_tld_with_space() {
        let manager = ConfigManager::new(false);
        let config = FileConfig {
            custom_presets: Some(HashMap::from([(
                "bad".to_string(),
                vec!["c om".to_string()],
            )])),
            ..Default::default()
        };
        let result = manager.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid TLD"));
    }

    #[test]
    fn test_validate_custom_preset_invalid_tld_empty_string() {
        let manager = ConfigManager::new(false);
        let config = FileConfig {
            custom_presets: Some(HashMap::from([("bad".to_string(), vec!["".to_string()])])),
            ..Default::default()
        };
        let result = manager.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid TLD"));
    }

    #[test]
    fn test_validate_valid_custom_preset() {
        let manager = ConfigManager::new(false);
        let config = FileConfig {
            custom_presets: Some(HashMap::from([(
                "mypreset".to_string(),
                vec!["com".to_string(), "org".to_string()],
            )])),
            ..Default::default()
        };
        assert!(manager.validate_config(&config).is_ok());
    }

    // ── merge_configs ───────────────────────────────────────────────────

    #[test]
    fn test_merge_defaults_higher_wins() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(10),
                preset: Some("startup".to_string()),
                pretty: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };
        let higher = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(25),
                pretty: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merged = manager.merge_configs(lower, higher);
        let defaults = merged.defaults.unwrap();
        assert_eq!(defaults.concurrency, Some(25));
        assert_eq!(defaults.preset, Some("startup".to_string()));
        assert_eq!(defaults.pretty, Some(true));
    }

    #[test]
    fn test_merge_defaults_lower_none() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig::default();
        let higher = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(50),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merged = manager.merge_configs(lower, higher);
        assert_eq!(merged.defaults.unwrap().concurrency, Some(50));
    }

    #[test]
    fn test_merge_defaults_higher_none() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(10),
                ..Default::default()
            }),
            ..Default::default()
        };
        let higher = FileConfig::default();

        let merged = manager.merge_configs(lower, higher);
        assert_eq!(merged.defaults.unwrap().concurrency, Some(10));
    }

    #[test]
    fn test_merge_defaults_both_none() {
        let manager = ConfigManager::new(false);
        let merged = manager.merge_configs(FileConfig::default(), FileConfig::default());
        assert!(merged.defaults.is_none());
    }

    #[test]
    fn test_merge_all_default_fields() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(10),
                preset: Some("lower".to_string()),
                tlds: Some(vec!["com".to_string()]),
                pretty: Some(false),
                timeout: Some("5s".to_string()),
                whois_fallback: Some(true),
                bootstrap: Some(false),
                detailed_info: Some(false),
            }),
            ..Default::default()
        };
        let higher = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(50),
                preset: Some("higher".to_string()),
                tlds: Some(vec!["org".to_string()]),
                pretty: Some(true),
                timeout: Some("30s".to_string()),
                whois_fallback: Some(false),
                bootstrap: Some(true),
                detailed_info: Some(true),
            }),
            ..Default::default()
        };

        let merged = manager.merge_configs(lower, higher);
        let d = merged.defaults.unwrap();
        assert_eq!(d.concurrency, Some(50));
        assert_eq!(d.preset, Some("higher".to_string()));
        assert_eq!(d.tlds, Some(vec!["org".to_string()]));
        assert_eq!(d.pretty, Some(true));
        assert_eq!(d.timeout, Some("30s".to_string()));
        assert_eq!(d.whois_fallback, Some(false));
        assert_eq!(d.bootstrap, Some(true));
        assert_eq!(d.detailed_info, Some(true));
    }

    #[test]
    fn test_merge_custom_presets_combined() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            custom_presets: Some(HashMap::from([
                ("a".to_string(), vec!["com".to_string()]),
                ("shared".to_string(), vec!["net".to_string()]),
            ])),
            ..Default::default()
        };
        let higher = FileConfig {
            custom_presets: Some(HashMap::from([
                ("b".to_string(), vec!["org".to_string()]),
                ("shared".to_string(), vec!["io".to_string()]),
            ])),
            ..Default::default()
        };

        let merged = manager.merge_configs(lower, higher);
        let presets = merged.custom_presets.unwrap();
        assert_eq!(presets.get("a"), Some(&vec!["com".to_string()]));
        assert_eq!(presets.get("b"), Some(&vec!["org".to_string()]));
        // Higher wins on conflict
        assert_eq!(presets.get("shared"), Some(&vec!["io".to_string()]));
    }

    #[test]
    fn test_merge_custom_presets_lower_none() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig::default();
        let higher = FileConfig {
            custom_presets: Some(HashMap::from([("a".to_string(), vec!["com".to_string()])])),
            ..Default::default()
        };
        let merged = manager.merge_configs(lower, higher);
        assert!(merged.custom_presets.is_some());
    }

    #[test]
    fn test_merge_custom_presets_higher_none() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            custom_presets: Some(HashMap::from([("a".to_string(), vec!["com".to_string()])])),
            ..Default::default()
        };
        let higher = FileConfig::default();
        let merged = manager.merge_configs(lower, higher);
        assert!(merged.custom_presets.is_some());
    }

    #[test]
    fn test_merge_monitoring_higher_wins() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            monitoring: Some(MonitoringConfig {
                interval: Some("5m".to_string()),
                notify_command: None,
            }),
            ..Default::default()
        };
        let higher = FileConfig {
            monitoring: Some(MonitoringConfig {
                interval: Some("10m".to_string()),
                notify_command: Some("echo done".to_string()),
            }),
            ..Default::default()
        };
        let merged = manager.merge_configs(lower, higher);
        let mon = merged.monitoring.unwrap();
        assert_eq!(mon.interval, Some("10m".to_string()));
        assert_eq!(mon.notify_command, Some("echo done".to_string()));
    }

    #[test]
    fn test_merge_output_higher_wins() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            output: Some(OutputConfig {
                default_format: Some("json".to_string()),
                csv_headers: Some(true),
                json_pretty: None,
            }),
            ..Default::default()
        };
        let higher = FileConfig {
            output: Some(OutputConfig {
                default_format: Some("csv".to_string()),
                csv_headers: None,
                json_pretty: Some(true),
            }),
            ..Default::default()
        };
        let merged = manager.merge_configs(lower, higher);
        let out = merged.output.unwrap();
        // monitoring/output use simple `or` — entire higher section replaces lower
        assert_eq!(out.default_format, Some("csv".to_string()));
    }

    #[test]
    fn test_merge_generation_higher_prefixes_win() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            generation: Some(GenerationConfig {
                prefixes: Some(vec!["get".to_string()]),
                suffixes: Some(vec!["hub".to_string()]),
            }),
            ..Default::default()
        };
        let higher = FileConfig {
            generation: Some(GenerationConfig {
                prefixes: Some(vec!["my".to_string(), "the".to_string()]),
                suffixes: None,
            }),
            ..Default::default()
        };

        let merged = manager.merge_configs(lower, higher);
        let gen = merged.generation.unwrap();
        assert_eq!(
            gen.prefixes,
            Some(vec!["my".to_string(), "the".to_string()])
        );
        assert_eq!(gen.suffixes, Some(vec!["hub".to_string()]));
    }

    #[test]
    fn test_merge_generation_both_none() {
        let manager = ConfigManager::new(false);
        let merged = manager.merge_configs(FileConfig::default(), FileConfig::default());
        assert!(merged.generation.is_none());
    }

    #[test]
    fn test_merge_generation_lower_none() {
        let manager = ConfigManager::new(false);
        let higher = FileConfig {
            generation: Some(GenerationConfig {
                prefixes: Some(vec!["get".to_string()]),
                suffixes: None,
            }),
            ..Default::default()
        };
        let merged = manager.merge_configs(FileConfig::default(), higher);
        assert!(merged.generation.is_some());
    }

    #[test]
    fn test_merge_generation_higher_none() {
        let manager = ConfigManager::new(false);
        let lower = FileConfig {
            generation: Some(GenerationConfig {
                prefixes: None,
                suffixes: Some(vec!["ly".to_string()]),
            }),
            ..Default::default()
        };
        let merged = manager.merge_configs(lower, FileConfig::default());
        assert_eq!(
            merged.generation.unwrap().suffixes,
            Some(vec!["ly".to_string()])
        );
    }

    // ── load_file with generation + output + monitoring ─────────────────

    #[test]
    fn test_load_generation_config() {
        let f = write_temp_config(
            r#"
[defaults]
concurrency = 20

[generation]
prefixes = ["get", "my"]
suffixes = ["hub", "ly"]
"#,
        );
        let manager = ConfigManager::new(false);
        let config = manager.load_file(f.path()).unwrap();
        let gen = config.generation.unwrap();
        assert_eq!(gen.prefixes, Some(vec!["get".into(), "my".into()]));
        assert_eq!(gen.suffixes, Some(vec!["hub".into(), "ly".into()]));
    }

    #[test]
    fn test_load_output_config() {
        let f = write_temp_config(
            r#"
[output]
default_format = "json"
csv_headers = true
json_pretty = false
"#,
        );
        let manager = ConfigManager::new(false);
        let config = manager.load_file(f.path()).unwrap();
        let out = config.output.unwrap();
        assert_eq!(out.default_format, Some("json".to_string()));
        assert_eq!(out.csv_headers, Some(true));
        assert_eq!(out.json_pretty, Some(false));
    }

    #[test]
    fn test_load_monitoring_config() {
        let f = write_temp_config(
            r#"
[monitoring]
interval = "5m"
notify_command = "echo done"
"#,
        );
        let manager = ConfigManager::new(false);
        let config = manager.load_file(f.path()).unwrap();
        let mon = config.monitoring.unwrap();
        assert_eq!(mon.interval, Some("5m".to_string()));
        assert_eq!(mon.notify_command, Some("echo done".to_string()));
    }

    // ── TOML serialization round-trip ───────────────────────────────────

    #[test]
    fn test_file_config_serialization_skip_none() {
        let config = FileConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        // All None fields should be skipped, resulting in empty-ish output
        assert!(!toml_str.contains("defaults"));
        assert!(!toml_str.contains("custom_presets"));
    }

    #[test]
    fn test_file_config_round_trip() {
        let config = FileConfig {
            defaults: Some(DefaultsConfig {
                concurrency: Some(25),
                preset: Some("tech".to_string()),
                ..Default::default()
            }),
            custom_presets: Some(HashMap::from([(
                "mine".to_string(),
                vec!["com".to_string(), "io".to_string()],
            )])),
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: FileConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.defaults.unwrap().concurrency, Some(25));
        assert!(parsed.custom_presets.unwrap().contains_key("mine"));
    }

    // ── EnvConfig methods ───────────────────────────────────────────────

    #[test]
    fn test_env_config_default() {
        let env = EnvConfig::default();
        assert!(env.concurrency.is_none());
        assert!(env.preset.is_none());
        assert!(env.tlds.is_none());
        assert!(env.pretty.is_none());
        assert!(env.timeout.is_none());
        assert!(env.json.is_none());
        assert!(env.csv.is_none());
        assert!(env.file.is_none());
        assert!(env.config.is_none());
        assert!(env.prefixes.is_none());
        assert!(env.suffixes.is_none());
    }

    #[test]
    fn test_get_effective_preset_no_tlds() {
        let env = EnvConfig {
            preset: Some("startup".to_string()),
            tlds: None,
            ..Default::default()
        };
        assert_eq!(env.get_effective_preset(), Some("startup".to_string()));
    }

    #[test]
    fn test_get_effective_preset_with_tlds_returns_none() {
        let env = EnvConfig {
            preset: Some("startup".to_string()),
            tlds: Some(vec!["com".to_string()]),
            ..Default::default()
        };
        // When explicit TLDs are set, preset is ignored
        assert_eq!(env.get_effective_preset(), None);
    }

    #[test]
    fn test_get_effective_preset_neither_set() {
        let env = EnvConfig::default();
        assert_eq!(env.get_effective_preset(), None);
    }

    #[test]
    fn test_get_effective_tlds() {
        let env = EnvConfig {
            tlds: Some(vec!["com".to_string(), "org".to_string()]),
            ..Default::default()
        };
        assert_eq!(
            env.get_effective_tlds(),
            Some(vec!["com".to_string(), "org".to_string()])
        );
    }

    #[test]
    fn test_get_effective_tlds_none() {
        let env = EnvConfig::default();
        assert_eq!(env.get_effective_tlds(), None);
    }

    #[test]
    fn test_has_output_format_conflict_both_true() {
        let env = EnvConfig {
            json: Some(true),
            csv: Some(true),
            ..Default::default()
        };
        assert!(env.has_output_format_conflict());
    }

    #[test]
    fn test_has_output_format_conflict_one_true() {
        let env = EnvConfig {
            json: Some(true),
            csv: Some(false),
            ..Default::default()
        };
        assert!(!env.has_output_format_conflict());
    }

    #[test]
    fn test_has_output_format_conflict_both_false() {
        let env = EnvConfig {
            json: Some(false),
            csv: Some(false),
            ..Default::default()
        };
        assert!(!env.has_output_format_conflict());
    }

    #[test]
    fn test_has_output_format_conflict_none() {
        let env = EnvConfig::default();
        assert!(!env.has_output_format_conflict());
    }

    #[test]
    fn test_has_output_format_conflict_one_none_one_true() {
        let env = EnvConfig {
            json: Some(true),
            csv: None,
            ..Default::default()
        };
        assert!(!env.has_output_format_conflict());
    }

    // ── load_env_config ─────────────────────────────────────────────────
    //
    // Env var mutations are process-global, so tests that touch DC_* vars
    // must be serialized. We use a mutex to prevent races.

    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_env_vars<F: FnOnce()>(vars: &[(&str, &str)], f: F) {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Clean slate: remove all DC_* vars first
        for key in &[
            "DC_CONCURRENCY",
            "DC_PRESET",
            "DC_TLD",
            "DC_PRETTY",
            "DC_TIMEOUT",
            "DC_WHOIS_FALLBACK",
            "DC_BOOTSTRAP",
            "DC_DETAILED_INFO",
            "DC_JSON",
            "DC_CSV",
            "DC_FILE",
            "DC_CONFIG",
            "DC_PREFIX",
            "DC_SUFFIX",
        ] {
            env::remove_var(key);
        }
        // Set requested vars
        for (k, v) in vars {
            env::set_var(k, v);
        }
        f();
        // Clean up
        for (k, _) in vars {
            env::remove_var(k);
        }
    }

    #[test]
    fn test_load_env_concurrency_valid() {
        with_env_vars(&[("DC_CONCURRENCY", "50")], || {
            let config = load_env_config(false);
            assert_eq!(config.concurrency, Some(50));
        });
    }

    #[test]
    fn test_load_env_concurrency_zero_ignored() {
        with_env_vars(&[("DC_CONCURRENCY", "0")], || {
            let config = load_env_config(false);
            assert!(config.concurrency.is_none());
        });
    }

    #[test]
    fn test_load_env_concurrency_over_100_ignored() {
        with_env_vars(&[("DC_CONCURRENCY", "200")], || {
            let config = load_env_config(false);
            assert!(config.concurrency.is_none());
        });
    }

    #[test]
    fn test_load_env_concurrency_non_numeric_ignored() {
        with_env_vars(&[("DC_CONCURRENCY", "abc")], || {
            let config = load_env_config(false);
            assert!(config.concurrency.is_none());
        });
    }

    #[test]
    fn test_load_env_preset() {
        with_env_vars(&[("DC_PRESET", "startup")], || {
            let config = load_env_config(false);
            assert_eq!(config.preset, Some("startup".to_string()));
        });
    }

    #[test]
    fn test_load_env_preset_empty_ignored() {
        with_env_vars(&[("DC_PRESET", "   ")], || {
            let config = load_env_config(false);
            assert!(config.preset.is_none());
        });
    }

    #[test]
    fn test_load_env_tld() {
        with_env_vars(&[("DC_TLD", "com,org,io")], || {
            let config = load_env_config(false);
            assert_eq!(
                config.tlds,
                Some(vec!["com".into(), "org".into(), "io".into()])
            );
        });
    }

    #[test]
    fn test_load_env_tld_with_spaces() {
        with_env_vars(&[("DC_TLD", " com , org , io ")], || {
            let config = load_env_config(false);
            assert_eq!(
                config.tlds,
                Some(vec!["com".into(), "org".into(), "io".into()])
            );
        });
    }

    #[test]
    fn test_load_env_tld_empty_entries_filtered() {
        with_env_vars(&[("DC_TLD", "com,,org,")], || {
            let config = load_env_config(false);
            assert_eq!(config.tlds, Some(vec!["com".into(), "org".into()]));
        });
    }

    #[test]
    fn test_load_env_pretty_true_variants() {
        for val in &["true", "1", "yes", "on", "TRUE", "Yes"] {
            with_env_vars(&[("DC_PRETTY", val)], || {
                let config = load_env_config(false);
                assert_eq!(
                    config.pretty,
                    Some(true),
                    "DC_PRETTY={} should be true",
                    val
                );
            });
        }
    }

    #[test]
    fn test_load_env_pretty_false_variants() {
        for val in &["false", "0", "no", "off", "FALSE", "No"] {
            with_env_vars(&[("DC_PRETTY", val)], || {
                let config = load_env_config(false);
                assert_eq!(
                    config.pretty,
                    Some(false),
                    "DC_PRETTY={} should be false",
                    val
                );
            });
        }
    }

    #[test]
    fn test_load_env_pretty_invalid_ignored() {
        with_env_vars(&[("DC_PRETTY", "maybe")], || {
            let config = load_env_config(false);
            assert!(config.pretty.is_none());
        });
    }

    #[test]
    fn test_load_env_timeout_valid() {
        with_env_vars(&[("DC_TIMEOUT", "30s")], || {
            let config = load_env_config(false);
            assert_eq!(config.timeout, Some("30s".to_string()));
        });
    }

    #[test]
    fn test_load_env_timeout_invalid_ignored() {
        with_env_vars(&[("DC_TIMEOUT", "invalid")], || {
            let config = load_env_config(false);
            assert!(config.timeout.is_none());
        });
    }

    #[test]
    fn test_load_env_boolean_flags() {
        with_env_vars(
            &[
                ("DC_WHOIS_FALLBACK", "true"),
                ("DC_BOOTSTRAP", "false"),
                ("DC_DETAILED_INFO", "1"),
                ("DC_JSON", "yes"),
                ("DC_CSV", "off"),
            ],
            || {
                let config = load_env_config(false);
                assert_eq!(config.whois_fallback, Some(true));
                assert_eq!(config.bootstrap, Some(false));
                assert_eq!(config.detailed_info, Some(true));
                assert_eq!(config.json, Some(true));
                assert_eq!(config.csv, Some(false));
            },
        );
    }

    #[test]
    fn test_load_env_file() {
        with_env_vars(&[("DC_FILE", "/path/to/domains.txt")], || {
            let config = load_env_config(false);
            assert_eq!(config.file, Some("/path/to/domains.txt".to_string()));
        });
    }

    #[test]
    fn test_load_env_file_empty_ignored() {
        with_env_vars(&[("DC_FILE", "  ")], || {
            let config = load_env_config(false);
            assert!(config.file.is_none());
        });
    }

    #[test]
    fn test_load_env_config_path() {
        with_env_vars(&[("DC_CONFIG", "/etc/dc.toml")], || {
            let config = load_env_config(false);
            assert_eq!(config.config, Some("/etc/dc.toml".to_string()));
        });
    }

    #[test]
    fn test_load_env_prefix_suffix() {
        with_env_vars(
            &[("DC_PREFIX", "get,my,try"), ("DC_SUFFIX", "hub,ly")],
            || {
                let config = load_env_config(false);
                assert_eq!(
                    config.prefixes,
                    Some(vec!["get".into(), "my".into(), "try".into()])
                );
                assert_eq!(config.suffixes, Some(vec!["hub".into(), "ly".into()]));
            },
        );
    }

    #[test]
    fn test_load_env_no_vars_returns_all_none() {
        with_env_vars(&[], || {
            let config = load_env_config(false);
            assert!(config.concurrency.is_none());
            assert!(config.preset.is_none());
            assert!(config.tlds.is_none());
            assert!(config.pretty.is_none());
            assert!(config.timeout.is_none());
            assert!(config.whois_fallback.is_none());
            assert!(config.bootstrap.is_none());
            assert!(config.detailed_info.is_none());
            assert!(config.json.is_none());
            assert!(config.csv.is_none());
            assert!(config.file.is_none());
            assert!(config.config.is_none());
            assert!(config.prefixes.is_none());
            assert!(config.suffixes.is_none());
        });
    }

    // ── ConfigManager verbose flag ──────────────────────────────────────

    #[test]
    fn test_config_manager_verbose_flag() {
        let manager = ConfigManager::new(true);
        assert!(manager.verbose);
        let manager = ConfigManager::new(false);
        assert!(!manager.verbose);
    }
}
