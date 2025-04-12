use clap::Parser;
use console::Style;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub domain: String,

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
    #[arg(short = 'w', long = "whois")]
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

    let re = regex::Regex::new(r"^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$").unwrap();

    if !re.is_match(domain) {
        return Err("Invalid domain format. Use something like 'example' or 'example.com'.".into());
    }

    Ok(domain.to_string())
}

/// Extracts the base name and TLD from a domain
fn extract_parts(domain: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), Some(parts[1].to_string()))
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
        ("zone", "https://rdap.nic.zone/domain/")
    ])
}

/// Dynamically finds RDAP endpoints for unknown TLDs using IANA bootstrap registry
async fn find_endpoint_for_tld(
    tld: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
                                            if let Some(url) = urls.get(0).and_then(|u| u.as_str())
                                            {
                                                return Ok(format!(
                                                    "{}/domain/",
                                                    url.trim_end_matches('/')
                                                ));
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

/// Checks domain availability using RDAP protocol
/// Returns (available, domain_info_json)
async fn check_rdap(
    domain: &str,
    endpoint_base: &str,
) -> Result<(bool, Option<serde_json::Value>), reqwest::Error> {
    let url = format!("{}{}", endpoint_base, domain);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let res = client.get(&url).send().await?;

    match res.status() {
        StatusCode::OK => {
            let json = res.json::<serde_json::Value>().await?;
            Ok((false, Some(json)))
        }
        StatusCode::NOT_FOUND => Ok((true, None)),
        _ => Ok((false, None)),
    }
}

/// Checks domain availability using WHOIS
/// Falls back to using command-line whois for reliability
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
    ];

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
                let is_registrar = roles
                    .iter()
                    .any(|role| role.as_str().map_or(false, |s| s == "registrar"));

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
                                            item_array.get(0).and_then(|f| f.as_str())
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

    // Extract dates
    if let Some(events) = json.get("events").and_then(|e| e.as_object()) {
        for (event_type, event_data) in events {
            match event_type.as_str() {
                "registration" => {
                    info.creation_date = event_data
                        .get("eventDate")
                        .and_then(|d| d.as_str())
                        .map(String::from);
                }
                "expiration" => {
                    info.expiration_date = event_data
                        .get("eventDate")
                        .and_then(|d| d.as_str())
                        .map(String::from);
                }
                _ => {}
            }
        }
    } else if let Some(events) = json.get("events").and_then(|e| e.as_array()) {
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
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut selected_index = 0;

    loop {
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
                "↑/↓: Navigate | Enter: View Details | s: Suggest Alternatives | q: Quit";
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL));

            f.render_widget(help, chunks[2]);
        })?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Up => {
                    if selected_index > 0 {
                        selected_index -= 1;
                    }
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

/// Controls the concurrency of domain checks with rate limiting
async fn check_domains_with_control(
    domains: Vec<String>,
    args: Args,
    registry_map: HashMap<&'static str, &'static str>,
) -> Vec<DomainStatus> {
    let max_concurrent = 5; // Limit concurrent requests
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let results = Arc::new(Mutex::new(Vec::new()));
    let endpoint_last_used = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));
    
    let mut handles = Vec::new();
    
    for domain in domains {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let registry_map = registry_map.clone();
        let results = Arc::clone(&results);
        let endpoint_last_used = Arc::clone(&endpoint_last_used);
        let args = args.clone();
        
        let handle = tokio::spawn(async move {
            // Check domain with timeout
            let status = tokio::time::timeout(
                Duration::from_secs(30), // Overall timeout for this domain check
                check_single_domain(&domain, &args, &registry_map, endpoint_last_used)
            ).await.unwrap_or_else(|_| {
                // Timeout occurred
                let status = DomainStatus {
                    domain: domain.clone(),
                    available: None,
                    info: None,
                };
                
                if args.debug {
                    println!("⚠️ Timeout occurred while checking {}", domain);
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
    registry_map: &HashMap<&'static str, &'static str>,
    endpoint_last_used: Arc<Mutex<HashMap<String, Instant>>>,
) -> DomainStatus {
    let parts: Vec<&str> = domain.split('.').collect();
    let tld = parts.last().unwrap_or(&"").to_string();
    
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
    if let Some(endpoint_base) = registry_map.get(tld.as_str()) {
        // Apply rate limiting for the same endpoint
        let endpoint = endpoint_base.to_string();
        let should_delay = {
            let last_used_map = endpoint_last_used.lock().unwrap();
            if let Some(last_time) = last_used_map.get(&endpoint) {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_millis(200) {
                    // Add a small delay if we used this endpoint recently
                    Some(Duration::from_millis(200) - elapsed)
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
        
        // Try RDAP check
        match check_rdap(domain, endpoint_base).await {
            Ok((true, _)) => {
                // Domain is available
                domain_status.available = Some(true);
                print_domain_available(domain, false, args, &green, &gray);
            }
            Ok((false, Some(json))) => {
                // Domain is taken with info
                domain_status.available = Some(false);
                domain_status.info = extract_domain_info(&json);
                print_domain_taken(domain, false, args, &domain_status.info, &red, &blue, &gray);
            }
            Ok((false, None)) => {
                // Domain is taken without info
                domain_status.available = Some(false);
                print_domain_taken(domain, false, args, &None, &red, &blue, &gray);
            }
            Err(e) => {
                // RDAP check failed
                if args.debug {
                    println!("⚠️ RDAP lookup failed for {}: {}", domain, e);
                }
                
                // Bootstrap or WHOIS fallback will be tried next
            }
        }
    } else if args.bootstrap && domain_status.available.is_none() {
        // Try IANA bootstrap for unknown TLDs
        if args.debug {
            println!("🔍 No known RDAP endpoint for .{}, trying bootstrap registry...", tld);
        }
        
        match find_endpoint_for_tld(&tld).await {
            Ok(endpoint_base) => match check_rdap(domain, &endpoint_base).await {
                Ok((true, _)) => {
                    domain_status.available = Some(true);
                    print_domain_available(domain, false, args, &green, &gray);
                }
                Ok((false, Some(json))) => {
                    domain_status.available = Some(false);
                    domain_status.info = extract_domain_info(&json);
                    print_domain_taken(domain, false, args, &domain_status.info, &red, &blue, &gray);
                }
                Ok((false, None)) => {
                    domain_status.available = Some(false);
                    print_domain_taken(domain, false, args, &None, &red, &blue, &gray);
                }
                Err(e) => {
                    if args.debug {
                        println!("⚠️ Bootstrap RDAP lookup failed for {}: {}", domain, e);
                    }
                }
            },
            Err(e) => {
                if args.debug {
                    println!("⚠️ Failed to find RDAP endpoint for .{}: {}", tld, e);
                }
            }
        }
    }
    
    // --- WHOIS Fallback ---
    // Use WHOIS if RDAP failed and WHOIS is not disabled
    if domain_status.available.is_none() && (!args.no_whois || args.whois_fallback) {
        if args.debug {
            println!("🔍 Trying WHOIS fallback for {}...", domain);
        }
        
        // Add a small delay to prevent rate limiting
        sleep(Duration::from_millis(100)).await;
        
        match check_whois(domain).await {
            Ok(available) => {
                domain_status.available = Some(available);
                if available {
                    print_domain_available(domain, true, args, &green, &gray);
                } else {
                    print_domain_taken(domain, true, args, &None, &red, &blue, &gray);
                }
            }
            Err(e) => {
                if args.debug {
                    println!("⚠️ WHOIS lookup failed for {}: {}", domain, e);
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

/// Prints a message indicating a domain is available
fn print_domain_available(
    domain: &str,
    via_whois: bool,
    args: &Args,
    green: &Style,
    gray: &Style,
) {
    let suffix = if via_whois && args.debug { " (via WHOIS)" } else { "" };
    
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
    let suffix = if via_whois && args.debug { " (via WHOIS)" } else { "" };
    
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

/// Main function to run the domain-check tool
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (base_name, tld_from_domain) = extract_parts(&args.domain);
    let domains = normalize_domains(&base_name, &args.tld, tld_from_domain);
    let registry_map = rdap_registry_map();

    if args.pretty {
        println!("🔍 Checking domain availability for: {}", base_name);
        println!(
            "🔍 With TLDs: {}\n",
            domains
                .iter()
                .map(|d| d.split('.').last().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if args.info {
            println!("ℹ️ Detailed info will be shown for taken domains\n");
        }
    }

    // Use concurrency-controlled domain checker
    let results = check_domains_with_control(domains, args.clone(), registry_map).await;

    if args.ui && !results.is_empty() {
        display_interactive_dashboard(&results)?;
    }

    if args.json {
        let json = serde_json::to_string_pretty(&results).unwrap();
        println!("\n{}", json);
    }

    Ok(())
}