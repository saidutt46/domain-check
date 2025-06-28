use clap::Parser;
use console::Style;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::sleep;

/// CLI arguments for domain-check
#[derive(Parser, Debug, Clone)]
#[command(name = "domain-check")]
#[command(version, author = "Sai Dutt G.V <gvs46@protonmail.com>")]
#[command(about = "Check domain availability using RDAP with WHOIS fallback", long_about = None)]
#[command(
    help_template = "{before-help}{name} {version}\n{author}\n{about}\n\n{usage-heading}\n  {usage}\n\n{all-args}{after-help}"
)]
pub struct Args {
    /// Domain name to check (without TLD for multiple TLD checking)
    #[arg(value_parser = validate_domain)]
    pub domain: Option<String>,

    /// Input file with domains to check (one per line)
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Max concurrent domain checks (default: 10, max: 100)
    #[arg(short = 'c', long = "concurrency", default_value = "10")]
    pub concurrency: usize,

    /// Override the 500 domain limit for bulk operations
    #[arg(long = "force")]
    pub force: bool,

    /// Check availability with these TLDs
    #[arg(short = 't', long = "tld", num_args = 1.., value_delimiter = ' ')]
    pub tld: Option<Vec<String>>,

    /// Output results in JSON format
    #[arg(short, long)]
    pub json: bool,

    /// Enable colorful, formatted output
    #[arg(short = 'p', long = "pretty")]
    pub pretty: bool,

    /// Show detailed domain information when available (for taken domains)
    #[arg(short = 'i', long = "info")]
    pub info: bool,

    /// Use IANA bootstrap to find RDAP endpoints for unknown TLDs
    #[arg(short = 'b', long = "bootstrap")]
    pub bootstrap: bool,

    /// Fallback to WHOIS when RDAP is unavailable (deprecated, enabled by default)
    #[arg(short = 'w', long = "whois", hide = true)]
    pub whois_fallback: bool,

    /// Launch interactive terminal UI dashboard
    #[arg(short = 'u', long = "ui")]
    pub ui: bool,

    /// Show detailed debug information and error messages
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    /// Disable automatic WHOIS fallback
    #[arg(long = "no-whois")]
    pub no_whois: bool,
}

/// Represents the status of a domain check
#[derive(Serialize, Deserialize, Clone, Debug)]
struct DomainStatus {
    domain: String,
    available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<DomainInfo>,
}

/// Detailed information about a domain
#[derive(Serialize, Deserialize, Clone, Debug)]
struct DomainInfo {
    registrar: Option<String>,
    creation_date: Option<String>,
    expiration_date: Option<String>,
    status: Vec<String>,
}

/// Validates the domain format to ensure it's a valid domain name
fn validate_domain(domain: &str) -> Result<String, String> {
    let domain = domain.trim();
    if domain.is_empty() {
        return Err("Domain name cannot be empty".into());
    }

    // Simplified regex that allows for IDNs and multi-level domains
    let re = regex::Regex::new(r"^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$").unwrap();

    if !re.is_match(domain) {
        return Err("Invalid domain format. Use something like 'example' or 'example.com'.".into());
    }

    Ok(domain.to_string())
}

/// Extracts the base name and TLD from a domain, handling multi-level TLDs
fn extract_parts(domain: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = domain.split('.').collect();

    if parts.len() >= 2 {
        // Handle multiple level TLDs by joining all parts after the first
        let base_name = parts[0].to_string();
        let tld = parts[1..].join(".");
        (base_name, Some(tld))
    } else {
        (domain.to_string(), None)
    }
}

/// Normalizes domain names based on the provided base name and TLDs
fn normalize_domains(
    base: &str,
    cli_tlds: &Option<Vec<String>>,
    extracted_tld: Option<String>,
) -> Vec<String> {
    match cli_tlds {
        Some(tlds) => tlds.iter().map(|tld| format!("{}.{}", base, tld)).collect(),
        None => {
            let tld = extracted_tld.unwrap_or_else(|| "com".to_string());
            vec![format!("{}.{}", base, tld)]
        }
    }
}

/// Returns a map of TLDs to their RDAP endpoints
fn rdap_registry_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        // gTLDs
        ("com", "https://rdap.verisign.com/com/v1/domain/"),
        ("net", "https://rdap.verisign.com/net/v1/domain/"),
        ("org", "https://rdap.pir.org/domain/"),
        ("info", "https://rdap.afilias.info/rdap/info/domain/"),
        ("biz", "https://rdap.nic.biz/domain/"),
        ("app", "https://rdap.nic.google/domain/"),
        ("dev", "https://rdap.nic.google/domain/"),
        ("page", "https://rdap.nic.google/domain/"),
        ("blog", "https://rdap.nic.blog/domain/"),
        ("shop", "https://rdap.nic.shop/domain/"),
        ("xyz", "https://rdap.nic.xyz/domain/"),
        ("tech", "https://rdap.nic.tech/domain/"),
        // ccTLDs
        ("io", "https://rdap.nic.io/domain/"),
        ("ai", "https://rdap.nic.ai/domain/"),
        ("co", "https://rdap.nic.co/domain/"),
        ("me", "https://rdap.nic.me/domain/"),
        ("us", "https://rdap.nic.us/domain/"),
        ("uk", "https://rdap.nominet.uk/domain/"),
        ("eu", "https://rdap.eu.org/domain/"),
        ("de", "https://rdap.denic.de/domain/"),
        ("ca", "https://rdap.cira.ca/domain/"),
        ("au", "https://rdap.auda.org.au/domain/"),
        ("fr", "https://rdap.nic.fr/domain/"),
        ("es", "https://rdap.nic.es/domain/"),
        ("it", "https://rdap.nic.it/domain/"),
        ("nl", "https://rdap.domain-registry.nl/domain/"),
        ("jp", "https://rdap.jprs.jp/domain/"),
        ("tv", "https://rdap.verisign.com/tv/v1/domain/"),
        ("cc", "https://rdap.verisign.com/cc/v1/domain/"),
        ("zone", "https://rdap.nic.zone/domain/"),
    ])
}

// Bootstrap registry cache
struct BootstrapCache {
    endpoints: HashMap<String, String>,
    last_update: Instant,
}

impl BootstrapCache {
    fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    fn get(&self, tld: &str) -> Option<String> {
        self.endpoints.get(tld).cloned()
    }

    fn insert(&mut self, tld: String, endpoint: String) {
        self.endpoints.insert(tld, endpoint);
        self.last_update = Instant::now();
    }

    fn is_stale(&self) -> bool {
        // Cache expires after 1 hour
        self.last_update.elapsed() > Duration::from_secs(3600)
    }
}

// Global bootstrap cache
lazy_static::lazy_static! {
    static ref BOOTSTRAP_CACHE: Mutex<BootstrapCache> = Mutex::new(BootstrapCache::new());
}

/// Dynamically finds RDAP endpoints for unknown TLDs using IANA bootstrap registry
async fn find_endpoint_for_tld(
    tld: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Check cache first
    {
        let cache = BOOTSTRAP_CACHE.lock().unwrap();
        if !cache.is_stale() {
            if let Some(endpoint) = cache.get(tld) {
                return Ok(endpoint);
            }
        }
    }

    // If not in cache or cache is stale, query IANA
    let bootstrap_url = "https://data.iana.org/rdap/dns.json";
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client.get(bootstrap_url).send().await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;

        if let Some(services) = json.get("services").and_then(|s| s.as_array()) {
            for service in services {
                if let Some(service_array) = service.as_array() {
                    if service_array.len() >= 2 {
                        if let Some(tlds) = service_array[0].as_array() {
                            for t in tlds {
                                if let Some(t_str) = t.as_str() {
                                    if t_str.to_lowercase() == tld.to_lowercase() {
                                        if let Some(urls) = service_array[1].as_array() {
                                            if let Some(url) = urls.first().and_then(|u| u.as_str())
                                            {
                                                let endpoint = format!(
                                                    "{}/domain/",
                                                    url.trim_end_matches('/')
                                                );

                                                // Update cache
                                                let mut cache = BOOTSTRAP_CACHE.lock().unwrap();
                                                cache.insert(tld.to_string(), endpoint.clone());

                                                return Ok(endpoint);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err("No RDAP endpoint found for this TLD".into())
}

/// Checks domain availability using RDAP protocol with timeout and retry
/// Returns (available, domain_info_json)
async fn check_rdap(
    domain: &str,
    endpoint_base: &str,
) -> Result<(bool, Option<serde_json::Value>), reqwest::Error> {
    let url = format!("{}{}", endpoint_base, domain);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5)) // Reduced timeout for faster response
        .build()?;

    // First attempt
    let res = client.get(&url).send().await?;

    match res.status() {
        StatusCode::OK => {
            let json = res.json::<serde_json::Value>().await?;
            Ok((false, Some(json)))
        }
        StatusCode::NOT_FOUND => Ok((true, None)),
        StatusCode::TOO_MANY_REQUESTS => {
            // Rate limited, retry after short delay
            sleep(Duration::from_millis(500)).await;
            let retry_res = client.get(&url).send().await?;

            match retry_res.status() {
                StatusCode::OK => {
                    let json = retry_res.json::<serde_json::Value>().await?;
                    Ok((false, Some(json)))
                }
                StatusCode::NOT_FOUND => Ok((true, None)),
                _ => Ok((false, None)),
            }
        }
        _ => Ok((false, None)),
    }
}

/// Checks domain availability using WHOIS with improved reliability
async fn check_whois(domain: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("whois")
        .arg(domain)
        .output()
        .await?;

    let output_text = String::from_utf8_lossy(&output.stdout).to_lowercase();

    // Common patterns that indicate domain availability
    let available_patterns = [
        "no match",
        "not found",
        "no data found",
        "no entries found",
        "domain not found",
        "domain available",
        "status: available",
        "status: free",
        "no information available",
        "not registered",
    ];

    // Common patterns for rate limiting or temporary failures
    let error_patterns = [
        "rate limit exceeded",
        "too many requests",
        "try again later",
        "quota exceeded",
    ];

    // Check for rate limiting first
    for pattern in &error_patterns {
        if output_text.contains(pattern) {
            // Retry after a delay if rate limited
            sleep(Duration::from_millis(1000)).await;

            let retry_output = tokio::process::Command::new("whois")
                .arg(domain)
                .output()
                .await?;

            let retry_text = String::from_utf8_lossy(&retry_output.stdout).to_lowercase();

            // Check available patterns after retry
            for available_pattern in available_patterns {
                if retry_text.contains(available_pattern) {
                    return Ok(true);
                }
            }

            return Ok(false);
        }
    }

    // Normal pattern checking
    for pattern in available_patterns {
        if output_text.contains(pattern) {
            return Ok(true);
        }
    }

    // If none of the "available" patterns matched, assume the domain is taken
    Ok(false)
}

/// Extracts domain information from RDAP response
fn extract_domain_info(json: &serde_json::Value) -> Option<DomainInfo> {
    let mut info = DomainInfo {
        registrar: None,
        creation_date: None,
        expiration_date: None,
        status: Vec::new(),
    };

    // Extract registrar - try multiple possible paths in RDAP response
    if let Some(entities) = json.get("entities").and_then(|e| e.as_array()) {
        for entity in entities {
            if let Some(roles) = entity.get("roles").and_then(|r| r.as_array()) {
                let is_registrar = roles.iter().any(|role| role.as_str() == Some("registrar"));

                if is_registrar {
                    // First try to get registrar name from vcardArray
                    if let Some(name) = entity
                        .get("vcardArray")
                        .and_then(|v| v.as_array())
                        .and_then(|a| a.get(1))
                        .and_then(|a| a.as_array())
                        .and_then(|items| {
                            for item in items {
                                if let Some(item_array) = item.as_array() {
                                    if item_array.len() >= 4 {
                                        if let Some(first) =
                                            item_array.first().and_then(|f| f.as_str())
                                        {
                                            if first == "fn" {
                                                return item_array
                                                    .get(3)
                                                    .and_then(|n| n.as_str())
                                                    .map(String::from);
                                            }
                                        }
                                    }
                                }
                            }
                            None
                        })
                    {
                        info.registrar = Some(name);
                        break;
                    }
                    // Then try publicIds
                    else if let Some(public_ids) =
                        entity.get("publicIds").and_then(|p| p.as_array())
                    {
                        if let Some(id) = public_ids
                            .first()
                            .and_then(|id| id.get("identifier"))
                            .and_then(|i| i.as_str())
                        {
                            info.registrar = Some(id.to_string());
                            break;
                        }
                    }
                    // Finally fall back to handle
                    else if let Some(handle) = entity.get("handle").and_then(|h| h.as_str()) {
                        info.registrar = Some(handle.to_string());
                        break;
                    }
                    // Or try to get the name directly
                    else if let Some(name) = entity.get("name").and_then(|n| n.as_str()) {
                        info.registrar = Some(name.to_string());
                        break;
                    }
                }
            }
        }
    }

    // Extract dates - Note: This only uses array form as the object form was incorrect in original code
    if let Some(events) = json.get("events").and_then(|e| e.as_array()) {
        for event in events {
            if let (Some(event_action), Some(event_date)) = (
                event.get("eventAction").and_then(|a| a.as_str()),
                event.get("eventDate").and_then(|d| d.as_str()),
            ) {
                match event_action {
                    "registration" => info.creation_date = Some(event_date.to_string()),
                    "expiration" => info.expiration_date = Some(event_date.to_string()),
                    _ => {}
                }
            }
        }
    }

    // Extract status
    if let Some(statuses) = json.get("status").and_then(|s| s.as_array()) {
        for status in statuses {
            if let Some(status_str) = status.as_str() {
                info.status.push(status_str.to_string());
            }
        }
    }

    Some(info)
}

/// Formats domain information for display
fn format_domain_info(info: &DomainInfo) -> String {
    let mut parts = Vec::new();

    if let Some(registrar) = &info.registrar {
        parts.push(format!("Registrar: {}", registrar));
    }

    if let Some(creation) = &info.creation_date {
        parts.push(format!("Created: {}", creation));
    }

    if let Some(expiration) = &info.expiration_date {
        parts.push(format!("Expires: {}", expiration));
    }

    if !info.status.is_empty() {
        parts.push(format!("Status: {}", info.status.join(", ")));
    }

    parts.join(" | ")
}

/// Displays an interactive terminal UI dashboard for domain status information
fn display_interactive_dashboard(
    domains: &[DomainStatus],
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use tui::{
        Terminal,
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    };

    // Setup SIGINT handler
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        tx.send(()).expect("Could not send signal on channel");
    })
    .expect("Error setting Ctrl-C handler");

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut selected_index = 0;

    loop {
        // Check if SIGINT received
        if rx.try_recv().is_ok() {
            break;
        }

        terminal.draw(|f| {
            let size = f.size();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3), // Title
                        Constraint::Min(10),   // Domains table
                        Constraint::Length(5), // Status bar & help
                    ]
                    .as_ref(),
                )
                .split(size);

            // Title
            let title = Paragraph::new("Domain Checker Dashboard")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Domain table
            let header_cells = ["Domain", "Status", "Registrar", "Created", "Expires"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
            let header = Row::new(header_cells).height(1).bottom_margin(1);

            let rows = domains.iter().enumerate().map(|(i, domain)| {
                let status_style = match domain.available {
                    Some(true) => Style::default().fg(Color::Green),
                    Some(false) => Style::default().fg(Color::Red),
                    None => Style::default().fg(Color::Gray),
                };

                let status_text = match domain.available {
                    Some(true) => "Available",
                    Some(false) => "Taken",
                    None => "Unknown",
                };

                // Create a default DomainInfo for unavailable domains
                let default_info = DomainInfo {
                    registrar: None,
                    creation_date: None,
                    expiration_date: None,
                    status: Vec::new(),
                };

                let info = domain.info.as_ref().unwrap_or(&default_info);

                let style = if i == selected_index {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                // Create cell content
                let registrar_text = info.registrar.clone().unwrap_or_else(|| "-".to_string());
                let creation_text = info
                    .creation_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string());
                let expiration_text = info
                    .expiration_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string());

                let cells = [
                    Cell::from(domain.domain.clone()),
                    Cell::from(status_text).style(status_style),
                    Cell::from(registrar_text),
                    Cell::from(creation_text),
                    Cell::from(expiration_text),
                ];

                Row::new(cells).style(style).height(1)
            });

            let domain_table = Table::new(rows)
                .header(header)
                .block(
                    Block::default()
                        .title("Domain Status")
                        .borders(Borders::ALL),
                )
                .widths(&[
                    Constraint::Percentage(30),
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                    Constraint::Percentage(17),
                    Constraint::Percentage(18),
                ]);

            f.render_widget(domain_table, chunks[1]);

            // Help text
            let help_text =
                "↑/↓: Navigate | Enter: View Details | s: Suggest Alternatives | q: Quit | Ctrl+C: Quit";
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL));

            f.render_widget(help, chunks[2]);
        })?;

        // Handle input with timeout to allow for SIGINT checks
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    // Handle Ctrl+C
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Up => {
                        selected_index = selected_index.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if selected_index < domains.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // View details functionality can be implemented here
                    }
                    KeyCode::Char('s') => {
                        if selected_index < domains.len() {
                            if let Some(false) = domains[selected_index].available {
                                // Suggestion functionality can be implemented here
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Processes domain checks in parallel with optimized concurrency control
async fn check_domains_in_parallel(
    domains: Vec<String>,
    args: Args,
    registry_map: HashMap<&'static str, &'static str>,
) -> Vec<DomainStatus> {
    let max_concurrent = args.concurrency.min(100); // Use the user-specified concurrency limit
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let results = Arc::new(Mutex::new(Vec::new()));
    let endpoint_last_used = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));

    // Use Arc to safely share the registry map
    let registry_map = Arc::new(registry_map);

    let mut handles = Vec::new();

    for domain in domains {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let registry_map = registry_map.clone(); // Clone the Arc, not the HashMap
        let results = Arc::clone(&results);
        let endpoint_last_used = Arc::clone(&endpoint_last_used);
        let args = args.clone();

        let handle = tokio::spawn(async move {
            // Check domain with timeout
            let status = tokio::time::timeout(
                Duration::from_secs(10), // Reduced from 30 to 10 seconds for faster response
                check_single_domain(&domain, &args, &registry_map, endpoint_last_used, false),
            )
            .await
            .unwrap_or_else(|_| {
                // Timeout occurred, switch to WHOIS fallback immediately
                if args.debug {
                    println!("⚠️ RDAP timeout for {}, trying WHOIS instead", domain);
                }

                let fallback_result =
                    tokio::runtime::Handle::current().block_on(check_whois(&domain));

                let available = fallback_result.unwrap_or(false);
                let status = DomainStatus {
                    domain: domain.clone(),
                    available: Some(available),
                    info: None,
                };

                // Print result directly
                if !args.json && !args.ui {
                    let green = Style::new().green().bold();
                    let red = Style::new().red().bold();
                    let gray = Style::new().dim();

                    if available {
                        if args.info {
                            println!(
                                "{} {} is AVAILABLE {}",
                                green.apply_to("🟢"),
                                domain,
                                gray.apply_to("(No info available for unregistered domains)")
                            );
                        } else {
                            println!("{} {} is AVAILABLE", green.apply_to("🟢"), domain);
                        }
                    } else if args.info {
                        println!(
                            "{} {} is TAKEN {}",
                            red.apply_to("🔴"),
                            domain,
                            gray.apply_to("(No info available via WHOIS fallback)")
                        );
                    } else {
                        println!("{} {} is TAKEN", red.apply_to("🔴"), domain);
                    }
                }

                status
            });

            // Store result
            let mut results_lock = results.lock().unwrap();
            results_lock.push(status);

            // Release the semaphore permit implicitly by dropping
            drop(permit);
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Return the collected results
    let results_lock = results.lock().unwrap();
    results_lock.clone()
}

/// Checks a single domain using RDAP with WHOIS fallback
async fn check_single_domain(
    domain: &str,
    args: &Args,
    registry_map: &Arc<HashMap<&'static str, &'static str>>,
    endpoint_last_used: Arc<Mutex<HashMap<String, Instant>>>,
    is_bulk_mode: bool,
) -> DomainStatus {
    let parts: Vec<&str> = domain.split('.').collect();
    let tld = if parts.len() >= 2 {
        parts.last().unwrap().to_string()
    } else {
        "".to_string()
    };

    let mut domain_status = DomainStatus {
        domain: domain.to_string(),
        available: None,
        info: None,
    };

    // Styles for output
    let green = Style::new().green().bold();
    let red = Style::new().red().bold();
    let blue = Style::new().blue();
    let gray = Style::new().dim();

    // --- RDAP Check ---
    let mut rdap_successful = false;

    if let Some(endpoint_base) = registry_map.get(tld.as_str()) {
        // Apply minimal rate limiting for the same endpoint
        let endpoint = endpoint_base.to_string();
        let should_delay = {
            let last_used_map = endpoint_last_used.lock().unwrap();
            if let Some(last_time) = last_used_map.get(&endpoint) {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_millis(100) {
                    // Reduced from 200ms to 100ms
                    Some(Duration::from_millis(100) - elapsed)
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(delay) = should_delay {
            sleep(delay).await;
        }

        // Update last used time for this endpoint
        {
            let mut last_used_map = endpoint_last_used.lock().unwrap();
            last_used_map.insert(endpoint.clone(), Instant::now());
        }

        // Try RDAP check with timeout
        match tokio::time::timeout(
            Duration::from_secs(3), // Short timeout to fail fast and try other methods
            check_rdap(domain, endpoint_base),
        )
        .await
        {
            Ok(Ok((true, _))) => {
                // Domain is available
                domain_status.available = Some(true);
                rdap_successful = true;
                if !is_bulk_mode {
                    print_domain_available(domain, false, args, &green, &gray);
                }
            }
            Ok(Ok((false, Some(json)))) => {
                // Domain is taken with info
                domain_status.available = Some(false);
                domain_status.info = extract_domain_info(&json);
                rdap_successful = true;
                if !is_bulk_mode {
                    print_domain_taken(
                        domain,
                        false,
                        args,
                        &domain_status.info,
                        &red,
                        &blue,
                        &gray,
                    );
                }
            }
            Ok(Ok((false, None))) => {
                // Domain is taken without info
                domain_status.available = Some(false);
                rdap_successful = true;
                if !is_bulk_mode {
                    print_domain_taken(domain, false, args, &None, &red, &blue, &gray);
                }
            }
            _ => {
                // RDAP check failed or timed out
                if args.debug {
                    println!("⚠️ RDAP lookup failed for {}", domain);
                }
            }
        }
    } else if args.bootstrap && !rdap_successful {
        // Try bootstrap with very short timeout
        if args.debug {
            println!(
                "🔍 No known RDAP endpoint for .{}, trying bootstrap registry...",
                tld
            );
        }

        match tokio::time::timeout(Duration::from_secs(2), find_endpoint_for_tld(&tld)).await {
            Ok(Ok(endpoint_base)) => {
                match tokio::time::timeout(
                    Duration::from_secs(3),
                    check_rdap(domain, &endpoint_base),
                )
                .await
                {
                    Ok(Ok((true, _))) => {
                        domain_status.available = Some(true);
                        rdap_successful = true;
                        if !is_bulk_mode {
                            print_domain_available(domain, false, args, &green, &gray);
                        }
                    }
                    Ok(Ok((false, Some(json)))) => {
                        domain_status.available = Some(false);
                        domain_status.info = extract_domain_info(&json);
                        rdap_successful = true;
                        if !is_bulk_mode {
                            print_domain_taken(
                                domain,
                                false,
                                args,
                                &domain_status.info,
                                &red,
                                &blue,
                                &gray,
                            );
                        }
                    }
                    Ok(Ok((false, None))) => {
                        domain_status.available = Some(false);
                        rdap_successful = true;
                        if !is_bulk_mode {
                            print_domain_taken(domain, false, args, &None, &red, &blue, &gray);
                        }
                    }
                    _ => {
                        if args.debug {
                            println!("⚠️ Bootstrap RDAP lookup failed for {}", domain);
                        }
                    }
                }
            }
            _ => {
                if args.debug {
                    println!("⚠️ Failed to find RDAP endpoint for .{}", tld);
                }
            }
        }
    }

    // --- WHOIS Fallback ---
    // Use WHOIS if RDAP failed and WHOIS is not disabled
    if !rdap_successful && (!args.no_whois || args.whois_fallback) {
        if args.debug {
            println!("🔍 Trying WHOIS fallback for {}...", domain);
        }

        // Skip rate limiting for WHOIS to be faster
        match tokio::time::timeout(Duration::from_secs(5), check_whois(domain)).await {
            Ok(Ok(available)) => {
                domain_status.available = Some(available);
                if !is_bulk_mode {
                    if available {
                        print_domain_available(domain, true, args, &green, &gray);
                    } else {
                        print_domain_taken(domain, true, args, &None, &red, &blue, &gray);
                    }
                }
            }
            _ => {
                if args.debug {
                    println!("⚠️ WHOIS lookup failed for {}", domain);
                }
            }
        }
    }

    // Final status check
    if domain_status.available.is_none() && args.debug {
        println!("⚠️ Could not determine availability for {}", domain);
    }

    domain_status
}

/// Validates a domain from a file line
fn validate_domain_line(line: &str, line_num: usize) -> Result<String, String> {
    let line = line.trim();

    // Skip empty lines or comment lines
    if line.is_empty() || line.starts_with('#') {
        return Err(format!(
            "Line {} is empty or a comment - skipping",
            line_num
        ));
    }

    // Handle trailing comments
    let domain_part = line.split('#').next().unwrap_or("").trim();
    if domain_part.is_empty() {
        return Err(format!(
            "Line {} contains only a comment - skipping",
            line_num
        ));
    }

    // Use existing validation logic
    validate_domain(domain_part)
}

/// Reads domains from a text file with enhanced validation
fn read_domains_from_file(
    file_path: &str,
    tlds: &Option<Vec<String>>,
    max_domains: usize,
    force: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
                // Skip empty or comment lines
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                match validate_domain_line(trimmed, line_num) {
                    Ok(domain) => {
                        // Check if domain already has a TLD
                        let parts: Vec<&str> = domain.split('.').collect();
                        if parts.len() >= 2 {
                            domains.push(domain); // Already has a TLD
                        } else if let Some(tld_list) = tlds {
                            // No TLD in the domain, use the TLDs from command line
                            for tld in tld_list {
                                domains.push(format!("{}.{}", domain, tld));
                            }
                        } else {
                            // No TLD in domain and no TLDs specified, default to .com
                            domains.push(format!("{}.com", domain));
                        }
                    }
                    Err(e) => {
                        invalid_lines.push(format!("Line {}: {} - {}", line_num, trimmed, e));
                    }
                }
            }
            Err(e) => {
                invalid_lines.push(format!("Line {}: Error reading line - {}", line_num, e));
            }
        }
    }

    // Check domain count limit
    if domains.len() > max_domains && !force {
        return Err(format!(
            "File contains {} domains, which exceeds the limit of {}. Use --force to override.",
            domains.len(),
            max_domains
        )
        .into());
    }

    // Log invalid lines
    if !invalid_lines.is_empty() {
        println!(
            "⚠️ Found {} invalid entries in the file:",
            invalid_lines.len()
        );
        for invalid in &invalid_lines[..invalid_lines.len().min(10)] {
            println!("  {}", invalid);
        }
        if invalid_lines.len() > 10 {
            println!(
                "  ... and {} more invalid entries",
                invalid_lines.len() - 10
            );
        }
        println!();
    }

    // Check if we have any valid domains
    if domains.is_empty() {
        return Err("No valid domains found in the file.".into());
    }

    Ok(domains)
}

/// Process bulk domain checks with optimized parallelism and streaming results
async fn check_domains_in_bulk(
    domains: Vec<String>,
    args: Args,
    registry_map: HashMap<&'static str, &'static str>,
) -> Vec<DomainStatus> {
    let max_concurrent = args.concurrency.min(100);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let results = Arc::new(Mutex::new(Vec::new()));
    let endpoint_last_used = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));

    // Track statistics for summary report
    let available_count = Arc::new(AtomicUsize::new(0));
    let taken_count = Arc::new(AtomicUsize::new(0));
    let unknown_count = Arc::new(AtomicUsize::new(0));

    // Print initial message for bulk check
    if !args.json && !args.ui && args.pretty {
        println!(
            "Starting bulk domain check with concurrency: {}",
            max_concurrent
        );
        println!("Results will stream as they complete:\n");
    }

    // Group domains by TLD to optimize rate limiting
    let mut domains_by_tld: HashMap<String, Vec<String>> = HashMap::new();

    for domain in domains {
        let parts: Vec<&str> = domain.split('.').collect();
        let tld = if parts.len() >= 2 {
            parts.last().unwrap().to_string()
        } else {
            "unknown".to_string()
        };

        domains_by_tld.entry(tld).or_default().push(domain);
    }

    // Use Arc to safely share the registry map
    let registry_map = Arc::new(registry_map);

    // Process domains grouped by TLD
    let mut handles = Vec::new();

    for (tld, domain_group) in domains_by_tld {
        // Check if this TLD exists in registry_map here, before spawning tasks
        let needs_rate_limiting = registry_map.contains_key(tld.as_str());

        // Process each group with slight delays between batches
        for domain in domain_group {
            // Add small delay between domains of same TLD to avoid rate limiting
            if needs_rate_limiting {
                sleep(Duration::from_millis(10)).await;
            }

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let registry_map = registry_map.clone();
            let results = Arc::clone(&results);
            let endpoint_last_used = Arc::clone(&endpoint_last_used);
            let available_count = Arc::clone(&available_count);
            let taken_count = Arc::clone(&taken_count);
            let unknown_count = Arc::clone(&unknown_count);
            let args = args.clone();

            let handle = tokio::spawn(async move {
                // Check domain with very strict timeout
                let status = tokio::time::timeout(
                    Duration::from_secs(8), // Even shorter timeout for bulk operations
                    check_single_domain(&domain, &args, &registry_map, endpoint_last_used, true),
                )
                .await
                .unwrap_or_else(|_| {
                    // Timeout occurred, switch to WHOIS fallback immediately
                    if args.debug {
                        println!("⚠️ Timeout for {}, trying WHOIS", domain);
                    }

                    let fallback_result =
                        tokio::runtime::Handle::current().block_on(check_whois(&domain));

                    let available = fallback_result.unwrap_or(false);

                    DomainStatus {
                        domain: domain.clone(),
                        available: Some(available),
                        info: None,
                    }
                });

                // Update statistics
                match status.available {
                    Some(true) => available_count.fetch_add(1, Ordering::Relaxed),
                    Some(false) => taken_count.fetch_add(1, Ordering::Relaxed),
                    None => unknown_count.fetch_add(1, Ordering::Relaxed),
                };

                // Print result directly using local style instances
                if !args.json && !args.ui {
                    // Create local style instances within the task
                    let green_local = Style::new().green().bold();
                    let red_local = Style::new().red().bold();
                    let blue_local = Style::new().blue();
                    let gray_local = Style::new().dim();

                    match status.available {
                        Some(true) => {
                            if args.info {
                                println!(
                                    "{} {} is AVAILABLE {}",
                                    green_local.apply_to("🟢"),
                                    domain,
                                    gray_local
                                        .apply_to("(No info available for unregistered domains)")
                                );
                            } else {
                                println!("{} {} is AVAILABLE", green_local.apply_to("🟢"), domain);
                            }
                        }
                        Some(false) => {
                            if args.info {
                                if let Some(domain_info) = &status.info {
                                    println!(
                                        "{} {} is TAKEN {}",
                                        red_local.apply_to("🔴"),
                                        domain,
                                        blue_local.apply_to(format_domain_info(domain_info))
                                    );
                                } else {
                                    println!(
                                        "{} {} is TAKEN {}",
                                        red_local.apply_to("🔴"),
                                        domain,
                                        gray_local.apply_to("(No info available)")
                                    );
                                }
                            } else {
                                println!("{} {} is TAKEN", red_local.apply_to("🔴"), domain);
                            }
                        }
                        None => println!("⚠️ {} status unknown", domain),
                    }
                }

                // Store result
                let mut results_lock = results.lock().unwrap();
                results_lock.push(status);

                // Release the semaphore permit implicitly by dropping
                drop(permit);
            });

            handles.push(handle);
        }
    }

    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Final statistics for summary
    let final_available = available_count.load(Ordering::Relaxed);
    let final_taken = taken_count.load(Ordering::Relaxed);
    let final_unknown = unknown_count.load(Ordering::Relaxed);

    // Return the collected results with statistics if needed
    let results_lock = results.lock().unwrap();
    let result_set = results_lock.clone();

    // Print summary if not in JSON or UI mode
    if !args.json && !args.ui && args.file.is_some() {
        println!(
            "\n✅ {} domains processed: 🟢 {} available, 🔴 {} taken, ⚠️ {} unknown",
            result_set.len(),
            final_available,
            final_taken,
            final_unknown
        );
    }

    result_set
}
/// Prints a message indicating a domain is available
fn print_domain_available(domain: &str, via_whois: bool, args: &Args, green: &Style, gray: &Style) {
    let suffix = if via_whois && args.debug {
        " (via WHOIS)"
    } else {
        ""
    };

    if args.info {
        println!(
            "{} {}{} is AVAILABLE {}",
            green.apply_to("🟢"),
            domain,
            suffix,
            gray.apply_to("(No info available for unregistered domains)")
        );
    } else {
        println!("{} {}{} is AVAILABLE", green.apply_to("🟢"), domain, suffix);
    }
}

/// Prints a message indicating a domain is taken
fn print_domain_taken(
    domain: &str,
    via_whois: bool,
    args: &Args,
    info: &Option<DomainInfo>,
    red: &Style,
    blue: &Style,
    gray: &Style,
) {
    let suffix = if via_whois && args.debug {
        " (via WHOIS)"
    } else {
        ""
    };

    if args.info {
        if via_whois {
            println!(
                "{} {}{} is TAKEN {}",
                red.apply_to("🔴"),
                domain,
                suffix,
                gray.apply_to("(Detailed info not available via WHOIS fallback)")
            );
        } else if let Some(domain_info) = info {
            println!(
                "{} {}{} is TAKEN {}",
                red.apply_to("🔴"),
                domain,
                suffix,
                blue.apply_to(format_domain_info(domain_info))
            );
        } else {
            println!(
                "{} {}{} is TAKEN {}",
                red.apply_to("🔴"),
                domain,
                suffix,
                gray.apply_to("(No info available)")
            );
        }
    } else {
        println!("{} {}{} is TAKEN", red.apply_to("🔴"), domain, suffix);
    }
}

// hello main
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let registry_map = rdap_registry_map();

    // Handle deprecated --whois flag with --no-whois flag
    if args.whois_fallback && args.no_whois {
        eprintln!(
            "Warning: Both --whois and --no-whois flags are set. --no-whois takes precedence."
        );
    }

    // Determine if we're in bulk mode or single domain mode
    let domains = if let Some(file_path) = &args.file {
        // Bulk mode from file
        read_domains_from_file(file_path, &args.tld, 500, args.force)?
    } else if let Some(domain_name) = &args.domain {
        // Single domain mode (original behavior)
        let (base_name, tld_from_domain) = extract_parts(domain_name);
        normalize_domains(&base_name, &args.tld, tld_from_domain)
    } else {
        // No domain or file specified
        return Err(
            "You must specify either a domain to check or a file with domains (--file)".into(),
        );
    };

    // Display appropriate messages based on mode and flags
    if args.file.is_some() {
        // Bulk mode messages
        if args.pretty {
            println!("🔍 Checking {} domains from file", domains.len());
        } else {
            println!("Checking {} domains from file...", domains.len());
        }

        // Add mode-specific info
        if args.ui {
            println!("Interactive UI will be shown after processing completes.");
        } else if args.json {
            println!("JSON results will be shown after processing completes.");
        }

        // Add concurrency info
        let concurrency = args.concurrency.min(100);
        println!("Using concurrency: {} - Please wait...\n", concurrency);

        if args.info && args.pretty {
            println!("ℹ️ Detailed info will be shown for taken domains\n");
        }
    } else {
        // Single domain mode messages
        if args.pretty {
            let base_name = domains
                .first()
                .map(|d| d.split('.').next().unwrap_or(""))
                .unwrap_or("");

            println!("🔍 Checking domain availability for: {}", base_name);

            let tlds_str = domains
                .iter()
                .filter_map(|d| d.split('.').nth(1))
                .collect::<Vec<_>>()
                .join(", ");

            println!("🔍 With TLDs: {}\n", tlds_str);

            if args.info {
                println!("ℹ️ Detailed info will be shown for taken domains\n");
            }
        } else {
            // Basic acknowledgment for single domain mode
            println!("Checking domain(s): {}\n", domains.join(", "));

            if args.ui {
                println!("Interactive UI will be shown after processing completes.");
            } else if args.json {
                println!("JSON results will be shown after processing completes.");
            }
        }
    }

    // Use the appropriate checker based on mode
    let results = if args.file.is_some() {
        check_domains_in_bulk(domains, args.clone(), registry_map).await
    } else {
        check_domains_in_parallel(domains, args.clone(), registry_map).await
    };

    // Generate output based on flags
    if args.ui && !results.is_empty() {
        display_interactive_dashboard(&results)?;
    } else if args.json {
        let json = serde_json::to_string_pretty(&results).unwrap();
        println!("\n{}", json);
    }

    Ok(())
}
