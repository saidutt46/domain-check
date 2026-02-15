//! Domain Check CLI Application
//!
//! A command-line interface for checking domain availability using RDAP and WHOIS protocols.
//! This CLI application provides a user-friendly interface to the domain-check-lib library.

mod ui;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::Parser;
use console::Term;
use domain_check_lib::{
    get_all_known_tlds, get_available_presets, get_preset_tlds, get_preset_tlds_with_custom,
    initialize_bootstrap,
};
use domain_check_lib::{load_env_config, ConfigManager, FileConfig};
use domain_check_lib::{CheckConfig, DomainChecker};
use std::io::BufRead;
use std::process;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

/// CLI arguments for domain-check
#[derive(Parser, Debug)]
#[command(name = "domain-check")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Sai Dutt G.V <gvs46@protonmail.com>")]
#[command(about = "Check domain availability using RDAP with WHOIS fallback")]
#[command(
    long_about = "Check domain availability using RDAP protocol with automatic WHOIS fallback.\n\nSupports concurrent checks, TLD presets, pattern generation, and multiple output formats."
)]
#[command(styles = STYLES)]
pub struct Args {
    /// Domain names to check (base names or FQDNs)
    #[arg(value_name = "DOMAINS", help_heading = "Domain Selection")]
    pub domains: Vec<String>,

    /// TLDs to check (comma-separated or multiple -t flags)
    #[arg(short = 't', long = "tld", value_name = "TLD", value_delimiter = ',', action = clap::ArgAction::Append, help_heading = "Domain Selection")]
    pub tlds: Option<Vec<String>>,

    /// Check against all known TLDs
    #[arg(long = "all", help_heading = "Domain Selection")]
    pub all_tlds: bool,

    /// Use a predefined TLD preset (use --list-presets to see all)
    #[arg(
        long = "preset",
        value_name = "NAME",
        help_heading = "Domain Selection"
    )]
    pub preset: Option<String>,

    /// List all available TLD presets and exit
    #[arg(long = "list-presets", help_heading = "Domain Selection")]
    pub list_presets: bool,

    /// Input file with domains (one per line)
    #[arg(
        short = 'f',
        long = "file",
        value_name = "FILE",
        help_heading = "Domain Selection"
    )]
    pub file: Option<String>,

    /// Pattern for name generation (\w=letter, \d=digit, ?=either)
    #[arg(
        long = "pattern",
        value_name = "PATTERN",
        value_delimiter = ',',
        help_heading = "Domain Generation"
    )]
    pub patterns: Option<Vec<String>>,

    /// Prefixes to prepend to domain names (comma-separated)
    #[arg(
        long = "prefix",
        value_name = "PREFIX",
        value_delimiter = ',',
        help_heading = "Domain Generation"
    )]
    pub prefixes: Option<Vec<String>>,

    /// Suffixes to append to domain names (comma-separated)
    #[arg(
        long = "suffix",
        value_name = "SUFFIX",
        value_delimiter = ',',
        help_heading = "Domain Generation"
    )]
    pub suffixes: Option<Vec<String>>,

    /// Preview generated domains without checking availability
    #[arg(long = "dry-run", help_heading = "Domain Generation")]
    pub dry_run: bool,

    /// Output results in JSON format
    #[arg(short = 'j', long = "json", help_heading = "Output Format")]
    pub json: bool,

    /// Output results in CSV format
    #[arg(long = "csv", help_heading = "Output Format")]
    pub csv: bool,

    /// Enable grouped, structured output with section headers
    #[arg(short = 'p', long = "pretty", help_heading = "Output Format")]
    pub pretty: bool,

    /// Show detailed domain information when available
    #[arg(short = 'i', long = "info", help_heading = "Output Format")]
    pub info: bool,

    /// Collect all results before displaying
    #[arg(long = "batch", help_heading = "Output Format")]
    pub batch: bool,

    /// Show results as they complete
    #[arg(long = "streaming", help_heading = "Output Format")]
    pub streaming: bool,

    /// Max concurrent domain checks (default: 20, max: 100)
    #[arg(
        short = 'c',
        long = "concurrency",
        default_value = "20",
        help_heading = "Performance"
    )]
    pub concurrency: usize,

    /// Override the 5000 domain limit for bulk operations
    #[arg(long = "force", help_heading = "Performance")]
    pub force: bool,

    /// Skip confirmation prompts (for automation/agents)
    #[arg(long = "yes", short = 'y', help_heading = "Performance")]
    pub yes: bool,

    /// Disable IANA bootstrap (use only hardcoded TLDs for RDAP)
    #[arg(long = "no-bootstrap", help_heading = "Protocol")]
    pub no_bootstrap: bool,

    /// Disable automatic WHOIS fallback
    #[arg(long = "no-whois", help_heading = "Protocol")]
    pub no_whois: bool,

    /// Use specific config file instead of automatic discovery
    #[arg(long = "config", value_name = "FILE", help_heading = "Configuration")]
    pub config: Option<String>,

    /// Show detailed debug information and error messages
    #[arg(short = 'd', long = "debug", help_heading = "Configuration")]
    pub debug: bool,

    /// Verbose logging
    #[arg(short = 'v', long = "verbose", help_heading = "Configuration")]
    pub verbose: bool,
}

/// Error statistics for aggregated reporting
#[derive(Debug, Default)]
pub(crate) struct ErrorStats {
    pub(crate) timeouts: Vec<String>,
    pub(crate) network_errors: Vec<String>,
    pub(crate) parsing_errors: Vec<String>,
    pub(crate) unknown_tld_errors: Vec<String>,
    pub(crate) other_errors: Vec<String>,
}

impl ErrorStats {
    fn add_error(&mut self, domain: &str, error: &domain_check_lib::DomainCheckError) {
        match error {
            domain_check_lib::DomainCheckError::Timeout { .. } => {
                self.timeouts.push(domain.to_string()); // Full domain name
            }
            domain_check_lib::DomainCheckError::NetworkError { .. } => {
                self.network_errors.push(domain.to_string());
            }
            domain_check_lib::DomainCheckError::ParseError { .. } => {
                self.parsing_errors.push(domain.to_string());
            }
            domain_check_lib::DomainCheckError::BootstrapError { .. } => {
                self.unknown_tld_errors.push(domain.to_string());
            }
            domain_check_lib::DomainCheckError::RdapError { .. } => {
                self.other_errors.push(domain.to_string());
            }
            domain_check_lib::DomainCheckError::WhoisError { .. } => {
                self.other_errors.push(domain.to_string());
            }
            _ => {
                self.other_errors.push(domain.to_string());
            }
        }
    }

    fn has_errors(&self) -> bool {
        !self.timeouts.is_empty()
            || !self.network_errors.is_empty()
            || !self.parsing_errors.is_empty()
            || !self.unknown_tld_errors.is_empty()
            || !self.other_errors.is_empty()
    }

    #[cfg(test)]
    fn format_summary(&self, args: &Args) -> String {
        if !self.has_errors() {
            return String::new();
        }

        let mut summary = vec!["⚠️  Some domains could not be checked:".to_string()];

        // Helper function to format domain list with smart truncation
        let format_domain_list = |domains: &[String], max_show: usize| -> String {
            if domains.len() <= max_show {
                domains.join(", ")
            } else {
                let shown = &domains[..max_show];
                let remaining = domains.len() - max_show;
                format!("{}, ... and {} more", shown.join(", "), remaining)
            }
        };

        if !self.timeouts.is_empty() {
            let domain_list = format_domain_list(&self.timeouts, 5); // Show max 5, then "and X more"
            summary.push(format!(
                "• {} timeouts: {}",
                self.timeouts.len(),
                domain_list
            ));
        }

        if !self.network_errors.is_empty() {
            let domain_list = format_domain_list(&self.network_errors, 5);
            summary.push(format!(
                "• {} network errors: {}",
                self.network_errors.len(),
                domain_list
            ));
        }

        if !self.parsing_errors.is_empty() {
            let domain_list = format_domain_list(&self.parsing_errors, 5);
            summary.push(format!(
                "• {} parsing errors: {}",
                self.parsing_errors.len(),
                domain_list
            ));
        }

        if !self.unknown_tld_errors.is_empty() {
            let domain_list = format_domain_list(&self.unknown_tld_errors, 5);
            summary.push(format!(
                "• {} unknown TLD errors: {}",
                self.unknown_tld_errors.len(),
                domain_list
            ));
        }

        if !self.other_errors.is_empty() {
            let domain_list = format_domain_list(&self.other_errors, 5);
            summary.push(format!(
                "• {} other errors: {}",
                self.other_errors.len(),
                domain_list
            ));
        }

        // Add retry information in debug mode
        if args.debug && self.has_errors() {
            summary.push("• All errors attempted WHOIS fallback where possible".to_string());
        }

        summary.join("\n")
    }
}

// HELPER FUNCTION to categorize errors from error messages
fn categorize_error_from_message(error_msg: &str) -> domain_check_lib::DomainCheckError {
    let msg_lower = error_msg.to_lowercase();

    if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        domain_check_lib::DomainCheckError::timeout(
            "domain check",
            std::time::Duration::from_secs(3),
        )
    } else if msg_lower.contains("network")
        || msg_lower.contains("dns")
        || msg_lower.contains("connect")
    {
        domain_check_lib::DomainCheckError::network("network error")
    } else if msg_lower.contains("parse") || msg_lower.contains("json") {
        domain_check_lib::DomainCheckError::ParseError {
            message: "parsing error".to_string(),
            content: None,
        }
    } else if msg_lower.contains("unknown")
        || msg_lower.contains("tld")
        || msg_lower.contains("bootstrap")
    {
        domain_check_lib::DomainCheckError::bootstrap("unknown", "unknown TLD")
    } else {
        domain_check_lib::DomainCheckError::internal("other error")
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Validate arguments
    if let Err(e) = validate_args(&args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // Handle --list-presets early
    if args.list_presets {
        print_presets();
        return;
    }

    // Set up logging if verbose
    if args.verbose {
        println!(
            "🔧 Domain Check CLI v{} starting...",
            env!("CARGO_PKG_VERSION")
        );
    }

    // Run the domain checking
    if let Err(e) = run_domain_check(args).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Validate command line arguments
fn validate_args(args: &Args) -> Result<(), String> {
    // --list-presets is self-contained, skip other validation
    if args.list_presets {
        return Ok(());
    }

    // Must have either domains, file, or patterns
    if args.domains.is_empty() && args.file.is_none() && args.patterns.is_none() {
        return Err(
            "You must specify domain names, a file with --file, or patterns with --pattern"
                .to_string(),
        );
    }

    // Can't have conflicting output modes
    if args.batch && args.streaming {
        return Err("Cannot specify both --batch and --streaming modes".to_string());
    }

    // Can't have multiple output formats
    let output_formats = [args.json, args.csv].iter().filter(|&&x| x).count();
    if output_formats > 1 {
        return Err("Cannot specify multiple output formats (--json, --csv)".to_string());
    }

    // Streaming mode doesn't support structured output formats
    if args.streaming && (args.json || args.csv) {
        return Err(
            "Cannot use --streaming with --json or --csv. Use --batch for structured output"
                .to_string(),
        );
    }

    // Validate concurrency
    if args.concurrency == 0 || args.concurrency > 100 {
        return Err("Concurrency must be between 1 and 100".to_string());
    }

    // Check for conflicting flags
    let tld_sources = [args.tlds.is_some(), args.preset.is_some(), args.all_tlds]
        .iter()
        .filter(|&&x| x)
        .count();

    if tld_sources > 1 {
        return Err(
            "Cannot specify multiple TLD sources. Use only one of: -t/--tld, --preset, or --all"
                .to_string(),
        );
    }

    Ok(())
}

/// Print all available TLD presets with their TLDs, then exit.
fn print_presets() {
    use console::Style;

    let heading = Style::new().yellow().bold();
    let name_style = Style::new().green().bold();
    let count_style = Style::new().cyan();

    println!();
    println!("{}", heading.apply_to("Available TLD Presets:"));
    println!();

    for preset_name in get_available_presets() {
        if let Some(tlds) = get_preset_tlds(preset_name) {
            let tld_list = tlds.join(", ");
            println!(
                "  {} {}  {}",
                name_style.apply_to(format!("{:<12}", preset_name)),
                count_style.apply_to(format!("({})", tlds.len())),
                tld_list,
            );
        }
    }

    println!();
    println!("Use: domain-check <name> --preset <preset>");
}

/// Determine if bootstrap should be enabled.
///
/// Bootstrap is now enabled by default. It can be disabled with `--no-bootstrap`.
fn should_enable_bootstrap(args: &Args, _resolved_tlds: &Option<Vec<String>>) -> bool {
    if args.no_bootstrap {
        return false;
    }
    // Bootstrap is enabled by default; --bootstrap flag is now a no-op
    true
}

/// Main domain checking logic
async fn run_domain_check(mut args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // Pre-warm bootstrap cache if --all mode is requested (so get_all_known_tlds()
    // returns the full ~1,180 TLDs from IANA, not just the 32 hardcoded ones)
    if args.all_tlds && !args.no_bootstrap {
        if args.verbose {
            println!("Fetching IANA bootstrap registry for full TLD coverage...");
        }
        if let Err(e) = initialize_bootstrap().await {
            if args.verbose {
                eprintln!(
                    "Warning: Bootstrap fetch failed ({}), using hardcoded TLDs",
                    e
                );
            }
            // Graceful degradation: continue with hardcoded 32 TLDs
        }
    }

    // Build configuration from CLI args
    let config = build_config(&args)?;

    // Propagate resolved config values back to args for display logic.
    // This ensures config/env settings for --info are respected in output formatting.
    args.info = config.detailed_info;

    // Determine domains to check (pass the config instead of rebuilding)
    let domains = get_domains_to_check(&args, &config).await?;

    // Dry-run: print domains and exit without checking
    if args.dry_run {
        if args.json {
            println!("{}", serde_json::to_string_pretty(&domains)?);
        } else {
            for d in &domains {
                println!("{}", d);
            }
        }
        eprintln!("{} domains would be checked", domains.len());
        return Ok(());
    }

    // Interactive confirmation for large runs (TTY only)
    if domains.len() > 5000 && !args.force && !args.yes {
        let term = Term::stderr();
        if term.is_term() {
            let estimated_secs = (domains.len() as f64 / config.concurrency as f64) * 1.0;
            eprint!(
                "Will check {} domains (~{:.0}s at concurrency {}). Proceed? [Y/n] ",
                domains.len(),
                estimated_secs,
                config.concurrency
            );
            let mut input = String::new();
            std::io::stdin().lock().read_line(&mut input)?;
            let answer = input.trim().to_lowercase();
            if answer == "n" || answer == "no" {
                eprintln!("Aborted.");
                return Ok(());
            }
        }
    }

    // Create domain checker
    let checker = DomainChecker::with_config(config.clone());

    // Decide on processing mode based on domain count and user preferences
    let use_streaming = should_use_streaming(&args, domains.len());

    if use_streaming {
        // Streaming mode for multiple domains - show progress and real-time results
        run_streaming_check(&checker, &domains, &args, &config.tlds).await?;
    } else {
        // Batch mode for single domains or when explicitly requested
        run_batch_check(&checker, &domains, &args).await?;
    }

    Ok(())
}

/// Determine whether to use streaming or batch mode
fn should_use_streaming(args: &Args, domain_count: usize) -> bool {
    // Force batch mode if explicitly requested
    if args.batch {
        return false;
    }

    // Force streaming mode if explicitly requested
    if args.streaming {
        return true;
    }

    // Use streaming for multiple domains unless in JSON/CSV mode
    if domain_count > 1 && !args.json && !args.csv {
        return true;
    }

    // Default to batch mode for single domains or structured output
    false
}

/// Run domain check in streaming mode with real-time progress
async fn run_streaming_check(
    checker: &DomainChecker,
    domains: &[String],
    args: &Args,
    tlds: &Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures::StreamExt;

    // Show initial progress message
    if args.pretty {
        ui::print_header(domains.len(), checker.config().concurrency, args);
    } else if args.verbose {
        println!(
            "🔍 Checking {} domains with concurrency: {}",
            domains.len(),
            checker.config().concurrency
        );

        if args.debug {
            println!("🔧 Domains: {}", domains.join(", "));
        }

        // Show TLD information
        if !args.json && !args.csv {
            if args.all_tlds {
                let tld_count = get_all_known_tlds().len();
                println!("🌐 Checking against all {} known TLDs", tld_count);
            } else if let Some(preset) = &args.preset {
                if let Some(tld_list) = tlds {
                    println!("🎯 Using '{}' preset ({} TLDs)", preset, tld_list.len());
                } else {
                    println!("🎯 Using '{}' preset", preset);
                }
            }
        }

        println!(); // Empty line for readability
    }

    // Track statistics for summary
    let mut available_count = 0;
    let mut taken_count = 0;
    let mut unknown_count = 0;
    let mut results = Vec::new();
    let mut error_stats = ErrorStats::default();
    let mut completed = 0usize;
    let total = domains.len();

    let start_time = std::time::Instant::now();

    // Process each domain individually to preserve context
    let domain_futures = domains.iter().map(|domain| {
        let domain = domain.clone();
        let checker = checker.clone();
        async move {
            match checker.check_domain(&domain).await {
                Ok(result) => result,
                Err(e) => domain_check_lib::DomainResult {
                    domain: domain.clone(),
                    available: None,
                    info: None,
                    check_duration: None,
                    method_used: domain_check_lib::CheckMethod::Unknown,
                    error_message: Some(e.to_string()),
                },
            }
        }
    });

    // Use buffer_unordered to maintain concurrency while preserving domain context
    let mut stream =
        futures::stream::iter(domain_futures).buffer_unordered(checker.config().concurrency);

    // Process results as they complete
    while let Some(domain_result) = stream.next().await {
        // Update statistics
        match domain_result.available {
            Some(true) => available_count += 1,
            Some(false) => taken_count += 1,
            None => {
                unknown_count += 1;
                if let Some(error_msg) = &domain_result.error_message {
                    let mock_error = categorize_error_from_message(error_msg);
                    error_stats.add_error(&domain_result.domain, &mock_error);
                }
            }
        }

        completed += 1;

        // Show result immediately
        let counter = if total > 1 {
            Some((completed, total))
        } else {
            None
        };
        if args.pretty {
            ui::print_result(&domain_result, args.info, args.debug, counter);
        } else {
            ui::print_result_default(&domain_result, args.info, args.debug, counter);
        }
        results.push(domain_result);
    }

    let duration = start_time.elapsed();

    // Show final summary for multiple domains
    if domains.len() > 1 && !args.json && !args.csv {
        println!();
        ui::print_summary(
            results.len(),
            available_count,
            taken_count,
            unknown_count,
            duration,
        );
        if error_stats.has_errors() {
            println!();
            ui::print_error_summary(&error_stats, args);
        }
    }

    Ok(())
}

/// Run domain check in batch mode (collect all results first)
async fn run_batch_check(
    checker: &DomainChecker,
    domains: &[String],
    args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_structured = args.json || args.csv;

    // Show header (pretty only — default mode lets the spinner + summary speak)
    if args.pretty && !is_structured && domains.len() > 1 {
        ui::print_header(domains.len(), checker.config().concurrency, args);
    } else if domains.len() > 1 && args.verbose {
        println!("🔍 Checking {} domains...", domains.len());
        if args.all_tlds {
            let tld_count = get_all_known_tlds().len();
            println!("🌐 Checking against all {} known TLDs", tld_count);
        } else if let Some(preset) = &args.preset {
            if let Some(preset_tlds) = get_preset_tlds(preset) {
                println!("🎯 Using '{}' preset ({} TLDs)", preset, preset_tlds.len());
            }
        }
    }

    // Start spinner for batch mode with multiple domains (all text modes).
    // Spinner::start returns None if stderr isn't a TTY.
    let spinner = if !is_structured && domains.len() > 1 {
        ui::Spinner::start(format!("Checking {} domains...", domains.len()))
    } else {
        None
    };

    let start_time = std::time::Instant::now();

    // Check all domains (concurrent under the hood)
    let results = checker.check_domains(domains).await?;

    let duration = start_time.elapsed();

    // Stop spinner before printing results
    if let Some(s) = spinner {
        s.stop().await;
    }

    // Display results based on format
    display_results(&results, args, duration)?;

    Ok(())
}

/// Build CheckConfig from CLI arguments with config file integration.
///
/// Precedence order (highest to lowest):
/// 1. CLI arguments (explicit user input)
/// 2. Environment variables (DC_*)  
/// 3. Local config file (./.domain-check.toml)
/// 4. Global config file (~/.domain-check.toml)
/// 5. XDG config file (~/.config/domain-check/config.toml)
/// 6. Built-in defaults
fn build_config(args: &Args) -> Result<CheckConfig, Box<dyn std::error::Error>> {
    let mut config = CheckConfig::default();

    // Create config manager for file discovery
    let config_manager = ConfigManager::new(args.verbose);

    // Step 1: Determine config file path and load config files
    if let Some(explicit_config_path) = &args.config {
        // CLI --config flag provided
        if args.verbose {
            println!(
                "🔧 Using explicit config file (CLI --config): {}",
                explicit_config_path
            );
        }

        let file_config = config_manager
            .load_file(explicit_config_path)
            .map_err(|e| {
                format!(
                    "Failed to load config file '{}': {}",
                    explicit_config_path, e
                )
            })?;

        config = merge_file_config_into_check_config(config, file_config);
    } else if let Ok(env_config_path) = std::env::var("DC_CONFIG") {
        // DC_CONFIG environment variable provided
        if args.verbose {
            println!(
                "🔧 Using explicit config file (DC_CONFIG env var): {}",
                env_config_path
            );
        }

        let file_config = config_manager
            .load_file(&env_config_path)
            .map_err(|e| format!("Failed to load config file '{}': {}", env_config_path, e))?;

        config = merge_file_config_into_check_config(config, file_config);
    } else {
        // No explicit config: Use automatic discovery
        if args.verbose {
            println!("🔧 Discovering config files...");
        }

        match config_manager.discover_and_load() {
            Ok(file_config) => {
                config = merge_file_config_into_check_config(config, file_config);
            }
            Err(e) if args.verbose => {
                eprintln!("⚠️ Config discovery warning: {}", e);
            }
            Err(_) => {
                // Silently continue with defaults if no config files found
            }
        }
    }

    // Step 2: Apply environment variables (DC_*)
    config = apply_environment_config(config, args.verbose);

    // Step 3: Apply CLI arguments (highest precedence)
    config = apply_cli_args_to_config(config, args)?;

    Ok(config)
}

/// Merge FileConfig into CheckConfig
fn merge_file_config_into_check_config(
    mut config: CheckConfig,
    file_config: FileConfig,
) -> CheckConfig {
    if let Some(defaults) = file_config.defaults {
        // Apply defaults from config file (only if not already set)
        if let Some(concurrency) = defaults.concurrency {
            config.concurrency = concurrency;
        }
        if let Some(whois_fallback) = defaults.whois_fallback {
            config.enable_whois_fallback = whois_fallback;
        }
        if let Some(bootstrap) = defaults.bootstrap {
            config.enable_bootstrap = bootstrap;
        }
        if let Some(detailed_info) = defaults.detailed_info {
            config.detailed_info = detailed_info;
        }

        // Handle TLDs and presets with proper precedence
        if let Some(tlds) = defaults.tlds {
            // Explicit TLD list wins over preset
            config.tlds = Some(tlds);
        } else if let Some(preset_name) = defaults.preset {
            // Convert preset name to TLD list
            // Note: Custom presets will be applied later in the config merge process
            if let Some(preset_tlds) = get_preset_tlds(&preset_name) {
                config.tlds = Some(preset_tlds);
            }
        }

        // Apply timeout settings
        if let Some(timeout_str) = defaults.timeout {
            if let Ok(timeout_secs) = parse_timeout_string(&timeout_str) {
                config.timeout = std::time::Duration::from_secs(timeout_secs);
                config.rdap_timeout = std::time::Duration::from_secs(timeout_secs.min(8));
                config.whois_timeout = std::time::Duration::from_secs(timeout_secs);
            }
        }
    }

    // Apply custom presets
    if let Some(custom_presets) = file_config.custom_presets {
        config.custom_presets = custom_presets;
    }

    config
}

/// Apply environment variables to config with comprehensive DC_* support.
///
/// Uses the library's load_env_config() for validation and proper handling.
fn apply_environment_config(mut config: CheckConfig, verbose: bool) -> CheckConfig {
    let env_config = load_env_config(verbose);

    // Check for output format conflicts
    if env_config.has_output_format_conflict() && verbose {
        eprintln!("⚠️ Both DC_JSON and DC_CSV are set to true, CLI args will resolve conflict");
    }

    // Apply environment config to CheckConfig
    if let Some(concurrency) = env_config.concurrency {
        config.concurrency = concurrency;
    }

    if let Some(whois_fallback) = env_config.whois_fallback {
        config.enable_whois_fallback = whois_fallback;
    }

    if let Some(bootstrap) = env_config.bootstrap {
        config.enable_bootstrap = bootstrap;
    }

    if let Some(detailed_info) = env_config.detailed_info {
        config.detailed_info = detailed_info;
    }

    // Handle TLD precedence: explicit TLDs > preset > config file values
    if let Some(tlds) = &env_config.tlds {
        config.tlds = Some(tlds.clone());
    } else if let Some(preset) = &env_config.preset {
        // Use custom presets if available, fall back to built-in
        if let Some(preset_tlds) = get_preset_tlds_with_custom(preset, Some(&config.custom_presets))
        {
            config.tlds = Some(preset_tlds);
        }
    }

    // Apply timeout if valid
    if let Some(timeout_str) = &env_config.timeout {
        if let Ok(timeout_secs) = parse_timeout_string(timeout_str) {
            config.timeout = std::time::Duration::from_secs(timeout_secs);
            config.rdap_timeout = std::time::Duration::from_secs(timeout_secs.min(8));
            config.whois_timeout = std::time::Duration::from_secs(timeout_secs);
        }
    }

    config
}

/// Apply CLI arguments to config (highest precedence).
///
/// CLI args override both environment variables and config file settings.
fn apply_cli_args_to_config(
    mut config: CheckConfig,
    args: &Args,
) -> Result<CheckConfig, Box<dyn std::error::Error>> {
    // CLI arguments always win over environment and config
    // Only override concurrency if explicitly provided by user
    // Note: We can't easily detect if clap default was used, so we check against default value
    // This is a limitation - if user explicitly sets --concurrency 20, it won't override env vars
    // But this is acceptable behavior (explicit same-as-default still counts as explicit)
    if args.concurrency != 20 {
        // 20 is the clap default
        config.concurrency = args.concurrency;
    }

    // Only override boolean settings when the user explicitly passes the flag.
    // Without this guard, the default (false) would always overwrite config/env values.
    if args.no_whois {
        config.enable_whois_fallback = false;
    }
    if args.info {
        config.detailed_info = true;
    }

    // Handle TLD precedence: CLI explicit > CLI preset > CLI all > env vars > config file
    if args.tlds.is_some() {
        config.tlds = args.tlds.clone();
    } else if let Some(preset) = &args.preset {
        // Use custom presets if available, fall back to built-in
        config.tlds = get_preset_tlds_with_custom(preset, Some(&config.custom_presets));
    } else if args.all_tlds {
        config.tlds = Some(get_all_known_tlds());
    }
    // Otherwise keep TLDs from environment or config file (already applied)

    // Bootstrap logic with environment consideration
    config.enable_bootstrap = should_enable_bootstrap(args, &config.tlds);

    Ok(config)
}

/// Parse timeout string like "5s", "30s", "2m" into seconds
fn parse_timeout_string(timeout_str: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let timeout_str = timeout_str.trim().to_lowercase();

    if timeout_str.ends_with('s') {
        timeout_str
            .strip_suffix('s')
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| "Invalid timeout format".into())
    } else if timeout_str.ends_with('m') {
        timeout_str
            .strip_suffix('m')
            .and_then(|s| s.parse::<u64>().ok())
            .map(|m| m * 60)
            .ok_or_else(|| "Invalid timeout format".into())
    } else {
        // Assume seconds if no unit
        timeout_str.parse::<u64>().map_err(|e| e.into())
    }
}

/// Get the list of domains to check from CLI args, environment, or file
async fn get_domains_to_check(
    args: &Args,
    config: &CheckConfig,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut base_names = Vec::new();

    // Step 1: Collect raw inputs from args and file
    base_names.extend(args.domains.clone());

    if let Some(cli_file) = &args.file {
        if args.verbose {
            println!("🔧 Reading domains from file (CLI --file): {}", cli_file);
        }
        let file_domains = read_domains_from_file(cli_file).await?;
        base_names.extend(file_domains);
    } else if let Ok(env_file_path) = std::env::var("DC_FILE") {
        if args.verbose {
            println!(
                "🔧 Reading domains from file (DC_FILE env var): {}",
                env_file_path
            );
        }
        let file_domains = read_domains_from_file(&env_file_path).await?;
        base_names.extend(file_domains);
    }

    // Step 2: Expand patterns into base names
    if let Some(patterns) = &args.patterns {
        for pattern in patterns {
            if args.verbose {
                let estimate = domain_check_lib::estimate_pattern_count(pattern)?;
                eprintln!("🔧 Pattern '{}' → ~{} names", pattern, estimate);
            }
            let expanded = domain_check_lib::expand_pattern(pattern)?;
            base_names.extend(expanded);
        }
    }

    // Step 3: Apply prefix/suffix permutations
    // CLI flags take priority; fall back to config file / env vars
    let config_prefixes = get_generation_prefixes(args);
    let config_suffixes = get_generation_suffixes(args);

    if config_prefixes.is_some() || config_suffixes.is_some() {
        let empty: Vec<String> = Vec::new();
        let prefixes = config_prefixes.as_deref().unwrap_or(&empty);
        let suffixes = config_suffixes.as_deref().unwrap_or(&empty);

        if args.verbose {
            if !prefixes.is_empty() {
                eprintln!("🔧 Prefixes: {}", prefixes.join(", "));
            }
            if !suffixes.is_empty() {
                eprintln!("🔧 Suffixes: {}", suffixes.join(", "));
            }
        }

        base_names =
            domain_check_lib::apply_affixes(&base_names, prefixes, suffixes, true).collect();
    }

    // Step 4: TLD expansion (existing, untouched)
    let expanded_domains = domain_check_lib::expand_domain_inputs(&base_names, &config.tlds);

    if expanded_domains.is_empty() {
        return Err("No valid domains found to check".into());
    }

    Ok(expanded_domains)
}

/// Load the generation config from config file, respecting --config flag
fn load_generation_config(args: &Args) -> Option<domain_check_lib::GenerationConfig> {
    let config_manager = ConfigManager::new(false);

    let file_config = if let Some(explicit_path) = &args.config {
        config_manager.load_file(explicit_path).ok()
    } else if let Ok(env_path) = std::env::var("DC_CONFIG") {
        config_manager.load_file(&env_path).ok()
    } else {
        config_manager.discover_and_load().ok()
    };

    file_config.and_then(|fc| fc.generation)
}

/// Get effective prefixes: CLI > env var (DC_PREFIX) > config file
fn get_generation_prefixes(args: &Args) -> Option<Vec<String>> {
    // CLI flags take highest priority
    if args.prefixes.is_some() {
        return args.prefixes.clone();
    }

    // Fall back to env var
    let env_config = load_env_config(false);
    if env_config.prefixes.is_some() {
        return env_config.prefixes;
    }

    // Fall back to config file
    if let Some(gen) = load_generation_config(args) {
        if gen.prefixes.is_some() {
            return gen.prefixes;
        }
    }

    None
}

/// Get effective suffixes: CLI > env var (DC_SUFFIX) > config file
fn get_generation_suffixes(args: &Args) -> Option<Vec<String>> {
    // CLI flags take highest priority
    if args.suffixes.is_some() {
        return args.suffixes.clone();
    }

    // Fall back to env var
    let env_config = load_env_config(false);
    if env_config.suffixes.is_some() {
        return env_config.suffixes;
    }

    // Fall back to config file
    if let Some(gen) = load_generation_config(args) {
        if gen.suffixes.is_some() {
            return gen.suffixes;
        }
    }

    None
}

/// Read domains from a file
async fn read_domains_from_file(
    file_path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    // Check if file exists
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path).into());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut domains = Vec::new();
    let mut invalid_lines = Vec::new();
    let mut line_num = 0;

    for line in reader.lines() {
        line_num += 1;
        match line {
            Ok(line) => {
                let trimmed = line.trim();

                // Skip empty lines and comments
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                // Handle inline comments
                let domain_part = trimmed.split('#').next().unwrap_or("").trim();
                if domain_part.is_empty() {
                    continue;
                }

                // Basic domain validation
                if domain_part.len() < 2 {
                    invalid_lines.push(format!(
                        "Line {}: '{}' - domain too short",
                        line_num, domain_part
                    ));
                    continue;
                }

                // Add domain (will be expanded later with TLDs if needed)
                domains.push(domain_part.to_string());
            }
            Err(e) => {
                invalid_lines.push(format!("Line {}: Error reading line - {}", line_num, e));
            }
        }
    }

    // Report invalid lines if any
    if !invalid_lines.is_empty() {
        eprintln!(
            "⚠️ Found {} invalid entries in the file:",
            invalid_lines.len()
        );
        for invalid in &invalid_lines[..invalid_lines.len().min(5)] {
            eprintln!("  {}", invalid);
        }
        if invalid_lines.len() > 5 {
            eprintln!("  ... and {} more invalid entries", invalid_lines.len() - 5);
        }
        eprintln!();
    }

    // Check if we have any valid domains
    if domains.is_empty() {
        return Err("No valid domains found in the file.".into());
    }

    Ok(domains)
}

fn display_results(
    results: &[domain_check_lib::DomainResult],
    args: &Args,
    duration: std::time::Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.json {
        display_json_results(results)?;
    } else if args.csv {
        display_csv_results(results)?;
    } else {
        display_text_results(results, args, duration)?;
    }

    Ok(())
}

/// Display results in JSON format
fn display_json_results(
    results: &[domain_check_lib::DomainResult],
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

/// Display results in CSV format
fn display_csv_results(
    results: &[domain_check_lib::DomainResult],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("domain,available,registrar,created,expires,method");

    for result in results {
        let available = match result.available {
            Some(true) => "true",
            Some(false) => "false",
            None => "unknown",
        };

        let registrar = result
            .info
            .as_ref()
            .and_then(|i| i.registrar.as_deref())
            .unwrap_or("-");

        let created = result
            .info
            .as_ref()
            .and_then(|i| i.creation_date.as_deref())
            .unwrap_or("-");

        let expires = result
            .info
            .as_ref()
            .and_then(|i| i.expiration_date.as_deref())
            .unwrap_or("-");

        println!(
            "{},{},{},{},{},{}",
            result.domain, available, registrar, created, expires, result.method_used
        );
    }

    Ok(())
}

/// Display results in human-readable text format
fn display_text_results(
    results: &[domain_check_lib::DomainResult],
    args: &Args,
    duration: std::time::Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.pretty {
        // Pretty mode: grouped layout with section headers
        ui::print_grouped_results(results, args.info, args.debug);
    } else {
        // Default mode: colored flat list
        for result in results {
            ui::print_result_default(result, args.info, args.debug, None);
        }
    }

    // Shared summary for both modes
    if results.len() > 1 {
        let available = results.iter().filter(|r| r.available == Some(true)).count();
        let taken = results
            .iter()
            .filter(|r| r.available == Some(false))
            .count();
        let unknown = results.iter().filter(|r| r.available.is_none()).count();
        println!();
        ui::print_summary(results.len(), available, taken, unknown, duration);
    }

    Ok(())
}

// domain-check/src/main.rs tests module

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function with all required fields
    fn create_test_args() -> Args {
        Args {
            domains: vec![], // Empty domains for testing
            tlds: None,
            file: None,
            config: None,
            concurrency: 20,
            force: false,
            info: false,
            no_whois: false,
            no_bootstrap: false,
            json: false,
            csv: false,
            pretty: false,
            batch: false,
            streaming: false,
            debug: false,
            verbose: false,
            all_tlds: false,
            preset: None,
            list_presets: false,
            patterns: None,
            prefixes: None,
            suffixes: None,
            dry_run: false,
            yes: false,
        }
    }

    #[test]
    fn test_should_enable_bootstrap_default() {
        // Bootstrap is now enabled by default
        let args = create_test_args();
        let tlds = Some(vec!["com".to_string(), "org".to_string()]);
        assert!(should_enable_bootstrap(&args, &tlds));
    }

    #[test]
    fn test_should_disable_bootstrap_with_flag() {
        // --no-bootstrap disables it
        let mut args = create_test_args();
        args.no_bootstrap = true;
        let tlds = Some(vec!["com".to_string(), "org".to_string()]);
        assert!(!should_enable_bootstrap(&args, &tlds));
    }

    #[test]
    fn test_categorize_error_from_message() {
        // Test timeout error categorization
        let timeout_error = categorize_error_from_message("Operation timed out after 3s");
        assert!(matches!(
            timeout_error,
            domain_check_lib::DomainCheckError::Timeout { .. }
        ));

        // Test network error categorization
        let network_error = categorize_error_from_message("dns error: failed to lookup");
        assert!(matches!(
            network_error,
            domain_check_lib::DomainCheckError::NetworkError { .. }
        ));

        // Test parsing error categorization
        let parse_error = categorize_error_from_message("Failed to parse JSON response");
        assert!(matches!(
            parse_error,
            domain_check_lib::DomainCheckError::ParseError { .. }
        ));

        // Test bootstrap error categorization
        let bootstrap_error = categorize_error_from_message("Unknown TLD not supported");
        assert!(matches!(
            bootstrap_error,
            domain_check_lib::DomainCheckError::BootstrapError { .. }
        ));
    }

    #[test]
    fn test_error_stats_aggregation() {
        let mut stats = ErrorStats::default();

        // Add different types of errors
        let timeout_error =
            domain_check_lib::DomainCheckError::timeout("test", std::time::Duration::from_secs(3));
        let network_error = domain_check_lib::DomainCheckError::network("network failure");

        stats.add_error("example.com", &timeout_error);
        stats.add_error("test.org", &network_error);
        stats.add_error("another.com", &timeout_error);

        // Verify aggregation
        assert_eq!(stats.timeouts.len(), 2);
        assert_eq!(stats.network_errors.len(), 1);
        assert!(stats.has_errors());

        // Verify domains are stored correctly
        assert!(stats.timeouts.contains(&"example.com".to_string()));
        assert!(stats.timeouts.contains(&"another.com".to_string()));
        assert!(stats.network_errors.contains(&"test.org".to_string()));
    }

    #[test]
    fn test_error_stats_format_summary() {
        let mut stats = ErrorStats::default();
        let args = create_test_args();

        // Test empty stats
        assert_eq!(stats.format_summary(&args), "");

        // Add some errors
        let timeout_error =
            domain_check_lib::DomainCheckError::timeout("test", std::time::Duration::from_secs(3));
        stats.add_error("example.com", &timeout_error);
        stats.add_error("test.org", &timeout_error);

        let summary = stats.format_summary(&args);
        assert!(summary.contains("⚠️  Some domains could not be checked:"));
        assert!(summary.contains("2 timeouts:"));
        assert!(summary.contains("example.com"));
        assert!(summary.contains("test.org"));
    }

    #[test]
    fn test_error_stats_truncation() {
        let mut stats = ErrorStats::default();
        let args = create_test_args();

        // Add more than 5 errors to test truncation
        let timeout_error =
            domain_check_lib::DomainCheckError::timeout("test", std::time::Duration::from_secs(3));
        for i in 0..8 {
            stats.add_error(&format!("domain{}.com", i), &timeout_error);
        }

        let summary = stats.format_summary(&args);
        assert!(summary.contains("8 timeouts:"));
        assert!(summary.contains("... and 3 more")); // Should truncate after 5
    }

    // validation tests to include required domains
    #[test]
    fn test_validate_args_invalid_preset_now_allowed() {
        // After Phase 4: Invalid presets are allowed in validate_args()
        // and checked later during config resolution
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()];
        args.preset = Some("invalid_preset".to_string());

        let result = validate_args(&args);
        assert!(result.is_ok()); // Now passes validation, fails later in config resolution
    }

    #[test]
    fn test_validate_args_conflicting_flags() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()]; // Add required domain
        args.tlds = Some(vec!["com".to_string()]);
        args.preset = Some("startup".to_string());

        let result = validate_args(&args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Cannot specify multiple TLD sources"));
    }

    #[test]
    fn test_validate_args_all_and_preset_conflict() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()]; // Add required domain
        args.all_tlds = true;
        args.preset = Some("startup".to_string());

        let result = validate_args(&args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Cannot specify multiple TLD sources"));
    }

    #[test]
    fn test_validate_args_valid_preset() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()]; // Add required domain
        args.preset = Some("startup".to_string());

        let result = validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_args_valid_all_flag() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()]; // Add required domain
        args.all_tlds = true;

        let result = validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_args_streaming_with_json_rejected() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()];
        args.streaming = true;
        args.json = true;

        let result = validate_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--streaming"));
    }

    #[test]
    fn test_validate_args_streaming_with_csv_rejected() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()];
        args.streaming = true;
        args.csv = true;

        let result = validate_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--streaming"));
    }

    #[test]
    fn test_validate_args_batch_with_json_allowed() {
        let mut args = create_test_args();
        args.domains = vec!["test".to_string()];
        args.batch = true;
        args.json = true;

        let result = validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_whois_flag_only_disables() {
        // When --no-whois is NOT passed, config/env values should be preserved
        let args = create_test_args(); // no_whois defaults to false
        let config = CheckConfig {
            enable_whois_fallback: false, // Simulates config setting
            ..CheckConfig::default()
        };

        let result = apply_cli_args_to_config(config, &args).unwrap();
        assert!(
            !result.enable_whois_fallback,
            "Config whois_fallback=false should be preserved when --no-whois is not passed"
        );
    }

    #[test]
    fn test_no_whois_flag_overrides_config() {
        // When --no-whois IS passed, it should disable whois regardless of config
        let mut args = create_test_args();
        args.no_whois = true;
        let config = CheckConfig {
            enable_whois_fallback: true,
            ..CheckConfig::default()
        };

        let result = apply_cli_args_to_config(config, &args).unwrap();
        assert!(
            !result.enable_whois_fallback,
            "--no-whois should disable whois fallback"
        );
    }

    #[test]
    fn test_info_flag_only_enables() {
        // When --info is NOT passed, config/env values should be preserved
        let args = create_test_args(); // info defaults to false
        let config = CheckConfig {
            detailed_info: true, // Simulates config setting
            ..CheckConfig::default()
        };

        let result = apply_cli_args_to_config(config, &args).unwrap();
        assert!(
            result.detailed_info,
            "Config detailed_info=true should be preserved when --info is not passed"
        );
    }

    #[test]
    fn test_info_flag_overrides_config() {
        // When --info IS passed, it should enable info regardless of config
        let mut args = create_test_args();
        args.info = true;
        let config = CheckConfig {
            detailed_info: false,
            ..CheckConfig::default()
        };

        let result = apply_cli_args_to_config(config, &args).unwrap();
        assert!(result.detailed_info, "--info should enable detailed info");
    }
}
