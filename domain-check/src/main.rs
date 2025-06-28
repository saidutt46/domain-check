//! Domain Check CLI Application
//!
//! A command-line interface for checking domain availability using RDAP and WHOIS protocols.
//! This CLI application provides a user-friendly interface to the domain-check-lib library.

use clap::Parser;
use domain_check_lib::{DomainChecker, CheckConfig};
use std::process;

/// CLI arguments for domain-check
#[derive(Parser, Debug)]
#[command(name = "domain-check")]
#[command(version = "0.4.0")]
#[command(author = "Sai Dutt G.V <gvs46@protonmail.com>")]
#[command(about = "Check domain availability using RDAP with WHOIS fallback")]
#[command(long_about = "A fast, robust CLI tool for checking domain availability using RDAP protocol with automatic WHOIS fallback and detailed domain information.")]
pub struct Args {
    /// Domain names to check (supports both base names and FQDNs)
    #[arg(value_name = "DOMAINS")]
    pub domains: Vec<String>,

    /// TLDs to check for base domain names
    #[arg(short = 't', long = "tld", value_name = "TLD")]
    pub tlds: Option<Vec<String>>,

    /// Input file with domains to check (one per line)
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<String>,

    /// Max concurrent domain checks (default: 10, max: 100)
    #[arg(short = 'c', long = "concurrency", default_value = "10")]
    pub concurrency: usize,

    /// Override the 500 domain limit for bulk operations
    #[arg(long = "force")]
    pub force: bool,

    /// Show detailed domain information when available
    #[arg(short = 'i', long = "info")]
    pub info: bool,

    /// Use IANA bootstrap to find RDAP endpoints for unknown TLDs
    #[arg(short = 'b', long = "bootstrap")]
    pub bootstrap: bool,

    /// Disable automatic WHOIS fallback
    #[arg(long = "no-whois")]
    pub no_whois: bool,

    /// Output results in JSON format
    #[arg(short = 'j', long = "json")]
    pub json: bool,

    /// Output results in CSV format
    #[arg(long = "csv")]
    pub csv: bool,

    /// Enable colorful, formatted output
    #[arg(short = 'p', long = "pretty")]
    pub pretty: bool,

    /// Force batch mode (collect all results first)
    #[arg(long = "batch")]
    pub batch: bool,

    /// Force streaming mode (show results as ready)
    #[arg(long = "streaming")]
    pub streaming: bool,

    /// Show detailed debug information and error messages
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    /// Verbose logging
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Validate arguments
    if let Err(e) = validate_args(&args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // Set up logging if verbose
    if args.verbose {
        println!("🔧 Domain Check CLI v0.4.0 starting...");
    }

    // Run the domain checking
    if let Err(e) = run_domain_check(args).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Validate command line arguments
fn validate_args(args: &Args) -> Result<(), String> {
    // Must have either domains or file
    if args.domains.is_empty() && args.file.is_none() {
        return Err("You must specify either domain names or a file with --file".to_string());
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

    // Validate concurrency
    if args.concurrency == 0 || args.concurrency > 100 {
        return Err("Concurrency must be between 1 and 100".to_string());
    }

    Ok(())
}

/// Main domain checking logic
async fn run_domain_check(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // Build configuration from CLI args
    let config = build_config(&args)?;

    // Create domain checker
    let checker = DomainChecker::with_config(config);

    // Determine domains to check
    let domains = get_domains_to_check(&args).await?;

    if args.verbose {
        println!("🔍 Checking {} domains with concurrency: {}", domains.len(), args.concurrency);
    }

    // Check domains based on output mode
    let results = checker.check_domains(&domains).await?;

    // Display results
    display_results(&results, &args)?;

    Ok(())
}

/// Build CheckConfig from CLI arguments
fn build_config(args: &Args) -> Result<CheckConfig, Box<dyn std::error::Error>> {
    let config = CheckConfig::default()
        .with_concurrency(args.concurrency)
        .with_whois_fallback(!args.no_whois)
        .with_bootstrap(args.bootstrap)
        .with_detailed_info(args.info);

    // Add TLDs if specified
    let config = if let Some(tlds) = &args.tlds {
        config.with_tlds(tlds.clone())
    } else {
        config
    };

    Ok(config)
}

/// Get the list of domains to check from CLI args or file
async fn get_domains_to_check(args: &Args) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut domains = Vec::new();

    // Add domains from command line
    domains.extend(args.domains.clone());

    // Add domains from file if specified
    if let Some(file_path) = &args.file {
        let file_domains = read_domains_from_file(file_path).await?;
        domains.extend(file_domains);
    }

    // Apply smart domain expansion
    let expanded_domains = domain_check_lib::expand_domain_inputs(&domains, &args.tlds);

    if expanded_domains.is_empty() {
        return Err("No valid domains found to check".into());
    }

    Ok(expanded_domains)
}

/// Read domains from a file
async fn read_domains_from_file(file_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // TODO: Implement file reading
    // For now, return an error to indicate it's not implemented
    Err(format!("File reading not implemented yet: {}", file_path).into())
}

/// Display results based on output format
fn display_results(results: &[domain_check_lib::DomainResult], args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.json {
        display_json_results(results)?;
    } else if args.csv {
        display_csv_results(results)?;
    } else {
        display_text_results(results, args)?;
    }

    Ok(())
}

/// Display results in JSON format
fn display_json_results(results: &[domain_check_lib::DomainResult]) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

/// Display results in CSV format
fn display_csv_results(results: &[domain_check_lib::DomainResult]) -> Result<(), Box<dyn std::error::Error>> {
    println!("domain,available,registrar,created,expires,method");
    
    for result in results {
        let available = match result.available {
            Some(true) => "true",
            Some(false) => "false", 
            None => "unknown",
        };
        
        let registrar = result.info.as_ref()
            .and_then(|i| i.registrar.as_deref())
            .unwrap_or("-");
            
        let created = result.info.as_ref()
            .and_then(|i| i.creation_date.as_deref())
            .unwrap_or("-");
            
        let expires = result.info.as_ref()
            .and_then(|i| i.expiration_date.as_deref())
            .unwrap_or("-");

        println!("{},{},{},{},{},{}", 
            result.domain, available, registrar, created, expires, result.method_used);
    }
    
    Ok(())
}

/// Display results in human-readable text format
fn display_text_results(results: &[domain_check_lib::DomainResult], args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut available_count = 0;
    let mut taken_count = 0;
    let mut unknown_count = 0;

    for result in results {
        match result.available {
            Some(true) => {
                available_count += 1;
                if args.pretty {
                    println!("🟢 {} is AVAILABLE", result.domain);
                } else {
                    println!("{} AVAILABLE", result.domain);
                }
            }
            Some(false) => {
                taken_count += 1;
                if args.info && result.info.is_some() {
                    let info = result.info.as_ref().unwrap();
                    if args.pretty {
                        println!("🔴 {} is TAKEN ({})", result.domain, 
                            format_domain_info(info));
                    } else {
                        println!("{} TAKEN ({})", result.domain, 
                            format_domain_info(info));
                    }
                } else {
                    if args.pretty {
                        println!("🔴 {} is TAKEN", result.domain);
                    } else {
                        println!("{} TAKEN", result.domain);
                    }
                }
            }
            None => {
                unknown_count += 1;
                if args.pretty {
                    println!("⚠️ {} status UNKNOWN", result.domain);
                } else {
                    println!("{} UNKNOWN", result.domain);
                }
            }
        }
    }

    // Show summary for multiple domains
    if results.len() > 1 {
        println!();
        if args.pretty {
            println!("✅ {} domains processed: 🟢 {} available, 🔴 {} taken, ⚠️ {} unknown", 
                results.len(), available_count, taken_count, unknown_count);
        } else {
            println!("Summary: {} available, {} taken, {} unknown", 
                available_count, taken_count, unknown_count);
        }
    }

    Ok(())
}

/// Format domain info for display
fn format_domain_info(info: &domain_check_lib::DomainInfo) -> String {
    let mut parts = Vec::new();

    if let Some(registrar) = &info.registrar {
        parts.push(format!("Registrar: {}", registrar));
    }

    if let Some(created) = &info.creation_date {
        parts.push(format!("Created: {}", created));
    }

    if let Some(expires) = &info.expiration_date {
        parts.push(format!("Expires: {}", expires));
    }

    if parts.is_empty() {
        "No info available".to_string()
    } else {
        parts.join(", ")
    }
}