use clap::Parser;
use reqwest::StatusCode;
use std::collections::HashMap;
use futures::future::join_all;
use console::Style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "domain-check")]
#[command(about = "Check domain availability using RDAP", long_about = None)]
pub struct Args {
    /// Domain name to check (without TLD, or full domain like example.com)
    #[arg(value_parser = validate_domain)]
    pub domain: String,

    /// TLDs to check (defaults to .com if not provided)
    #[arg(short = 't', long = "tld", num_args = 1.., value_delimiter = ' ')]
    pub tld: Option<Vec<String>>,

    /// Output result in JSON format
    #[arg(short, long)]
    pub json: bool,

    /// Output a pretty formatted view instead of raw/plain
    #[arg(short = 'p', long = "pretty")]
    pub pretty: bool,
}

/// Struct used for serializing JSON output
#[derive(Serialize)]
struct DomainStatus {
    domain: String,
    available: bool,
}

/// Validate the domain input format
fn validate_domain(domain: &str) -> Result<String, String> {
    let domain = domain.trim();

    if domain.is_empty() {
        return Err("Domain name cannot be empty".into());
    }

    let re = regex::Regex::new(r"^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)?$").unwrap();

    if !re.is_match(domain) {
        return Err("Invalid domain format. Use something like 'example' or 'example.com'.".into());
    }

    Ok(domain.to_string())
}

/// Splits a full domain like 'example.com' into ("example", "com")
fn extract_parts(domain: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), Some(parts[1].to_string()))
    } else {
        (domain.to_string(), None)
    }
}

/// Returns a list of domains to check, based on CLI input and fallback
fn normalize_domains(base: &str, cli_tlds: &Option<Vec<String>>, extracted_tld: Option<String>) -> Vec<String> {
    match cli_tlds {
        Some(tlds) => tlds.iter().map(|tld| format!("{}.{}", base, tld)).collect(),
        None => {
            let tld = extracted_tld.unwrap_or_else(|| "com".to_string());
            vec![format!("{}.{}", base, tld)]
        }
    }
}

/// Returns RDAP endpoint base URL for known TLDs
fn rdap_registry_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("com", "https://rdap.verisign.com/com/v1/domain/"),
        ("net", "https://rdap.verisign.com/net/v1/domain/"),
        ("org", "https://rdap.publicinterestregistry.net/rdap/org/domain/"),
        ("io", "https://rdap.nic.io/domain/"),
        ("app", "https://rdap.nic.google/domain/"),
        // Add more as needed
    ])
}

/// Check RDAP availability by querying the RDAP endpoint for a domain
async fn check_rdap(domain: &str, endpoint_base: &str) -> Result<bool, reqwest::Error> {
    let url = format!("{}{}", endpoint_base, domain);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let res = client.get(&url).send().await?;

    match res.status() {
        StatusCode::OK => Ok(false),        // Domain is taken
        StatusCode::NOT_FOUND => Ok(true),  // Domain is available
        _ => Ok(false),                     // Unexpected response → treat as unavailable
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (base_name, tld_from_domain) = extract_parts(&args.domain);
    let domain_list = normalize_domains(&base_name, &args.tld, tld_from_domain);
    let registry_map = rdap_registry_map();

    // Optional pretty output before checking
    if args.pretty {
        println!("\n🔍 Domains to check:");
        for (i, domain) in domain_list.iter().enumerate() {
            println!("  [{}] {}", i + 1, domain);
        }
        println!("\n🌐 Querying RDAP servers...\n");
    }

    // Spinner / Progress bar
    let pb = ProgressBar::new(domain_list.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Checking {pos}/{len} domains...")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Style for output
    let green = Style::new().green().bold();
    let red = Style::new().red().bold();
    let yellow = Style::new().yellow();

    // Spawn tasks
    let tasks = domain_list.iter().map(|domain| {
        let domain = domain.clone();
        let registry_map = registry_map.clone();

        tokio::spawn(async move {
            let parts: Vec<&str> = domain.split('.').collect();
            let tld = parts.last().unwrap_or(&"");

            if let Some(endpoint_base) = registry_map.get(tld) {
                let available = match check_rdap(&domain, endpoint_base).await {
                    Ok(true) => Some(true),
                    Ok(false) => Some(false),
                    Err(_) => None,
                };
                Some(DomainStatus { domain, available: available.unwrap_or(false) })
            } else {
                None
            }
        })
    });

    let results: Vec<DomainStatus> = join_all(tasks)
        .await
        .into_iter()
        .filter_map(|res| res.ok().flatten()) // unwrap tokio::JoinHandle and filter None
        .collect();

    pb.finish_with_message("✅ Done checking domains!");

    // Output results
    if args.json {
        // JSON output
        let json = serde_json::to_string_pretty(&results).unwrap();
        println!("{}", json);
    } else {
        // Pretty CLI output
        for result in results {
            if args.pretty {
                if result.available {
                    println!("{} {} is AVAILABLE", green.apply_to("✅"), result.domain);
                } else {
                    println!("{} {} is TAKEN", red.apply_to("❌"), result.domain);
                }
            } else {
                println!("{}: {}", result.domain, if result.available { "available" } else { "taken" });
            }
        }
    }
}
