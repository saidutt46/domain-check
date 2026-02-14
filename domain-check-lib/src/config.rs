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

    #[test]
    fn test_parse_timeout_string() {
        assert_eq!(parse_timeout_string("5s"), Some(5));
        assert_eq!(parse_timeout_string("30s"), Some(30));
        assert_eq!(parse_timeout_string("2m"), Some(120));
        assert_eq!(parse_timeout_string("5"), Some(5));
        assert_eq!(parse_timeout_string("invalid"), None);
    }

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
[defaults]
concurrency = 25
preset = "startup"
pretty = true

[custom_presets]
my_preset = ["com", "org", "io"]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let manager = ConfigManager::new(false);
        let config = manager.load_file(temp_file.path()).unwrap();

        assert!(config.defaults.is_some());
        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.concurrency, Some(25));
        assert_eq!(defaults.preset, Some("startup".to_string()));
        assert_eq!(defaults.pretty, Some(true));

        assert!(config.custom_presets.is_some());
        let presets = config.custom_presets.unwrap();
        assert_eq!(
            presets.get("my_preset"),
            Some(&vec![
                "com".to_string(),
                "org".to_string(),
                "io".to_string()
            ])
        );
    }

    #[test]
    fn test_invalid_concurrency() {
        let config_content = r#"
[defaults]
concurrency = 0
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let manager = ConfigManager::new(false);
        let result = manager.load_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_configs() {
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

        assert_eq!(defaults.concurrency, Some(25)); // Higher wins
        assert_eq!(defaults.preset, Some("startup".to_string())); // Lower preserved
        assert_eq!(defaults.pretty, Some(true)); // Higher wins
    }

    #[test]
    fn test_load_generation_config() {
        let config_content = r#"
[defaults]
concurrency = 20

[generation]
prefixes = ["get", "my"]
suffixes = ["hub", "ly"]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let manager = ConfigManager::new(false);
        let config = manager.load_file(temp_file.path()).unwrap();

        assert!(config.generation.is_some());
        let gen = config.generation.unwrap();
        assert_eq!(
            gen.prefixes,
            Some(vec!["get".to_string(), "my".to_string()])
        );
        assert_eq!(
            gen.suffixes,
            Some(vec!["hub".to_string(), "ly".to_string()])
        );
    }

    #[test]
    fn test_merge_generation_configs() {
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

        // Higher prefixes win
        assert_eq!(
            gen.prefixes,
            Some(vec!["my".to_string(), "the".to_string()])
        );
        // Lower suffixes preserved (higher didn't set)
        assert_eq!(gen.suffixes, Some(vec!["hub".to_string()]));
    }
}
