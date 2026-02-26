//! Display logic for domain-check CLI.
//!
//! This module handles all styled terminal output: colored result lines,
//! grouped batch output (--pretty), spinner animation, progress counters,
//! headers, and summaries. Uses only the `console` crate (already a dependency).
//!
//! Default mode: colored status words, progress counter, spinner, colored summary.
//! Pretty mode: everything above plus grouped layout, column alignment, styled header.

use console::{pad_str, style, Alignment, Term};
use domain_check_lib::{DomainInfo, DomainResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::Args;

// ── Custom help ─────────────────────────────────────────────────────────────

/// Print a fully custom help screen with ASCII art, compact flags, and examples.
pub fn print_custom_help() {
    // ASCII art banner — cyan bold
    let banner = r#"     _                       _                  _               _
  __| | ___  _ __ ___   __ _(_)_ __         ___| |__   ___  ___| | __
 / _` |/ _ \| '_ ` _ \ / _` | | '_ \ _____ / __| '_ \ / _ \/ __| |/ /
| (_| | (_) | | | | | | (_| | | | | |_____| (__| | | |  __/ (__|   <
 \__,_|\___/|_| |_| |_|\__,_|_|_| |_|      \___|_| |_|\___|\___|_|\_\"#;

    println!("{}", style(banner).cyan().bold());
    println!();
    println!(
        " {} {}",
        style("domain-check").white().bold(),
        style(format!("v{}", env!("CARGO_PKG_VERSION"))).dim(),
    );
    println!(
        " {}",
        style("Check domain availability using RDAP with WHOIS fallback").dim()
    );

    // USAGE
    print_section("USAGE");
    println!(
        "   {} {} {}",
        style("domain-check").cyan().bold(),
        style("[DOMAINS]").white(),
        style("[--flags]").dim()
    );
    println!(
        "   {} {} {}",
        style("domain-check").cyan().bold(),
        style("--file <FILE>").white(),
        style("[--flags]").dim()
    );
    println!(
        "   {} {} {}",
        style("domain-check").cyan().bold(),
        style("--pattern <PATTERN>").white(),
        style("[--flags]").dim()
    );

    // DOMAIN SELECTION
    print_section("DOMAIN SELECTION");
    print_flag(
        "",
        "[DOMAINS]",
        "Domain names to check (base names or FQDNs)",
    );
    print_flag(
        "-t",
        "--tld <TLD>",
        "TLDs to check (comma-separated or multiple -t)",
    );
    print_flag("", "--all", "Check against all known TLDs");
    print_flag("", "--preset <NAME>", "Use a predefined TLD preset");
    print_flag(
        "",
        "--list-presets",
        "List all available TLD presets and exit",
    );
    print_flag(
        "-f",
        "--file <FILE>",
        "Input file with domains (one per line)",
    );

    // DOMAIN GENERATION
    print_section("DOMAIN GENERATION");
    print_flag(
        "",
        "--pattern <PATTERN>",
        "Pattern for name generation (\\w=letter, \\d=digit, ?=either)",
    );
    print_flag(
        "",
        "--prefix <PREFIX>",
        "Prefixes to prepend (comma-separated)",
    );
    print_flag(
        "",
        "--suffix <SUFFIX>",
        "Suffixes to append (comma-separated)",
    );
    print_flag(
        "",
        "--dry-run",
        "Preview generated domains without checking",
    );

    // OUTPUT FORMAT
    print_section("OUTPUT FORMAT");
    print_flag("-j", "--json", "Output results in JSON format");
    print_flag("", "--csv", "Output results in CSV format");
    print_flag("-p", "--pretty", "Grouped output with section headers");
    print_flag("-i", "--info", "Show detailed domain information");
    print_flag("", "--batch", "Collect all results before displaying");
    print_flag("", "--streaming", "Show results as they complete");

    // PERFORMANCE
    print_section("PERFORMANCE");
    print_flag(
        "-c",
        "--concurrency <N>",
        "Max concurrent checks (default: 20, max: 100)",
    );
    print_flag("", "--force", "Override the 5000 domain limit");
    print_flag("-y", "--yes", "Skip confirmation prompts");

    // PROTOCOL
    print_section("PROTOCOL");
    print_flag(
        "",
        "--no-bootstrap",
        "Disable IANA bootstrap (hardcoded TLDs only)",
    );
    print_flag("", "--no-whois", "Disable automatic WHOIS fallback");

    // CONFIGURATION
    print_section("CONFIGURATION");
    print_flag("", "--config <FILE>", "Use specific config file");
    print_flag("-d", "--debug", "Show detailed debug info and errors");
    print_flag("-v", "--verbose", "Verbose logging");

    // GENERAL
    print_section("GENERAL");
    print_flag("-h", "--help", "Show this help message");
    print_flag("-V", "--version", "Show version");

    // EXAMPLES
    print_section("EXAMPLES");
    print_example("domain-check myapp", "Check myapp.com (default TLD)");
    print_example("domain-check myapp -t com,io,dev", "Check specific TLDs");
    print_example(
        "domain-check myapp --preset startup",
        "Use the startup TLD preset",
    );
    print_example(
        "domain-check --pattern \"app\\d\" --dry-run",
        "Preview pattern-generated names",
    );

    println!();
}

/// Print a magenta bold section header.
fn print_section(name: &str) {
    println!();
    println!(" {}", style(name).magenta().bold());
}

/// Print a compact flag line with aligned columns.
fn print_flag(short: &str, long: &str, desc: &str) {
    if short.is_empty() {
        // No short flag — 6 chars of padding
        println!("       {:<24} {}", style(long).cyan(), desc);
    } else {
        println!(
            "   {}  {:<24} {}",
            style(short).cyan(),
            style(long).cyan(),
            desc,
        );
    }
}

/// Print an example line with green command and dim description.
fn print_example(cmd: &str, desc: &str) {
    println!(
        "   {} {}",
        style(format!("$ {:<44}", cmd)).green(),
        style(desc).dim(),
    );
}

// ── Spinner ──────────────────────────────────────────────────────────────────

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// An async braille-dot spinner that writes to stderr so stdout stays clean.
pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Spinner {
    /// Start a new spinner with the given message (e.g. "Checking 8 domains...").
    ///
    /// Returns `None` if stderr is not a TTY (piped output, CI, etc.) to avoid
    /// polluting non-interactive streams with escape codes.
    /// Waits 500ms before showing to avoid a flash on fast operations.
    pub fn start(message: String) -> Option<Self> {
        let term = Term::stderr();
        if !term.is_term() {
            return None;
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let handle = tokio::spawn(async move {
            let term = Term::stderr();

            // Wait before showing — avoids spinner flash on fast operations
            tokio::time::sleep(Duration::from_millis(500)).await;

            let mut idx = 0usize;
            while running_clone.load(Ordering::Relaxed) {
                let frame = SPINNER_FRAMES[idx % SPINNER_FRAMES.len()];
                let _ = term.clear_line();
                let _ = term.write_str(&format!("{} {}", style(frame).cyan(), message));
                idx += 1;
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
            let _ = term.clear_line();
        });

        Some(Self {
            running,
            handle: Some(handle),
        })
    }

    /// Stop the spinner and clear the line.
    pub async fn stop(self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle {
            let _ = h.await;
        }
    }
}

// ── Header ───────────────────────────────────────────────────────────────────

/// Print a styled header at the start of a pretty run.
pub fn print_header(domain_count: usize, concurrency: usize, args: &Args) {
    println!(
        "{} {} {}",
        style("domain-check").bold(),
        style(format!("v{}", env!("CARGO_PKG_VERSION"))).dim(),
        style(format!(
            "— Checking {} domain{}",
            domain_count,
            if domain_count == 1 { "" } else { "s" }
        ))
        .dim(),
    );

    let mut meta_parts: Vec<String> = Vec::new();

    if let Some(preset) = &args.preset {
        meta_parts.push(format!("Preset: {}", preset));
    }
    if args.all_tlds {
        let tld_count = domain_check_lib::get_all_known_tlds().len();
        meta_parts.push(format!("All {} TLDs", tld_count));
    }
    meta_parts.push(format!("Concurrency: {}", concurrency));

    println!("{}", style(meta_parts.join(" | ")).dim());
    println!();
}

// ── Single result line ───────────────────────────────────────────────────────

/// Format and print a single domain result with colors and alignment.
///
/// If `counter` is Some((current, total)), a progress prefix like `[3/8]` is shown.
pub fn print_result(
    result: &DomainResult,
    show_info: bool,
    debug: bool,
    counter: Option<(usize, usize)>,
) {
    let domain_width = 30;
    let padded_domain = pad_str(&result.domain, domain_width, Alignment::Left, Some(".."));

    let prefix = match counter {
        Some((cur, total)) => {
            format!("{} ", style(format!("[{}/{}]", cur, total)).dim())
        }
        None => String::new(),
    };

    match result.available {
        Some(true) => {
            println!(
                "  {}{}  {}",
                prefix,
                style(&padded_domain).white(),
                style("AVAILABLE").green().bold(),
            );
        }
        Some(false) => {
            let info_str = if show_info {
                result
                    .info
                    .as_ref()
                    .map(|i| format!("  {}", style(format_domain_info(i)).dim()))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            println!(
                "  {}{}  {}{}",
                prefix,
                style(&padded_domain).white(),
                style("TAKEN").red().bold(),
                info_str,
            );
        }
        None => {
            let reason = brief_error(result);
            println!(
                "  {}{}  {}  {}",
                prefix,
                style(&padded_domain).white(),
                style("UNKNOWN").yellow(),
                style(reason).dim(),
            );
        }
    }

    if debug {
        if let Some(duration) = result.check_duration {
            println!(
                "    {} Checked in {}ms via {}",
                style("└─").dim(),
                duration.as_millis(),
                result.method_used,
            );
        }
    }
}

// ── Default result line (colored, flat) ───────────────────────────────────────

/// Print a single domain result with colored status words but flat layout.
/// No padding or column alignment — just `domain STATUS` with color.
///
/// If `counter` is Some((current, total)), a progress prefix like `[3/8]` is shown.
pub fn print_result_default(
    result: &DomainResult,
    show_info: bool,
    debug: bool,
    counter: Option<(usize, usize)>,
) {
    let prefix = match counter {
        Some((cur, total)) => format!("{} ", style(format!("[{}/{}]", cur, total)).dim()),
        None => String::new(),
    };

    match result.available {
        Some(true) => {
            println!(
                "{}{} {}",
                prefix,
                result.domain,
                style("AVAILABLE").green().bold(),
            );
        }
        Some(false) => {
            let info_str = if show_info {
                result
                    .info
                    .as_ref()
                    .map(|i| format!(" ({})", style(format_domain_info(i)).dim()))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            println!(
                "{}{} {}{}",
                prefix,
                result.domain,
                style("TAKEN").red().bold(),
                info_str,
            );
        }
        None => {
            let reason = brief_error(result);
            println!(
                "{}{} {} {}",
                prefix,
                result.domain,
                style("UNKNOWN").yellow(),
                style(reason).dim(),
            );
        }
    }

    if debug {
        if let Some(duration) = result.check_duration {
            println!(
                "    {} Checked in {}ms via {}",
                style("└─").dim(),
                duration.as_millis(),
                result.method_used,
            );
        }
    }
}

// ── Grouped batch output (Issue #17 core) ────────────────────────────────────

/// Print results grouped by status: Available, Taken, Unknown.
/// Empty sections are omitted entirely.
pub fn print_grouped_results(results: &[DomainResult], show_info: bool, debug: bool) {
    let mut available: Vec<&DomainResult> = Vec::new();
    let mut taken: Vec<&DomainResult> = Vec::new();
    let mut unknown: Vec<&DomainResult> = Vec::new();

    for r in results {
        match r.available {
            Some(true) => available.push(r),
            Some(false) => taken.push(r),
            None => unknown.push(r),
        }
    }

    if !available.is_empty() {
        println!(
            "  {} {}",
            style(format!("── Available ({}) ", available.len()))
                .green()
                .bold(),
            style("─".repeat(40)).green().dim(),
        );
        for r in &available {
            print_grouped_line(r, show_info, debug);
        }
        println!();
    }

    if !taken.is_empty() {
        println!(
            "  {} {}",
            style(format!("── Taken ({}) ", taken.len())).red().bold(),
            style("─".repeat(44)).red().dim(),
        );
        for r in &taken {
            print_grouped_line(r, show_info, debug);
        }
        println!();
    }

    if !unknown.is_empty() {
        println!(
            "  {} {}",
            style(format!("── Unknown ({}) ", unknown.len()))
                .yellow()
                .bold(),
            style("─".repeat(40)).yellow().dim(),
        );
        for r in &unknown {
            print_grouped_line(r, show_info, debug);
        }
        println!();
    }
}

/// Print a single line inside a grouped section.
fn print_grouped_line(result: &DomainResult, show_info: bool, debug: bool) {
    let domain_width = 30;
    let padded = pad_str(&result.domain, domain_width, Alignment::Left, Some(".."));

    match result.available {
        Some(true) => {
            println!("    {}", style(&padded).white());
        }
        Some(false) => {
            let info_str = if show_info {
                result
                    .info
                    .as_ref()
                    .map(|i| format!("  {}", style(format_domain_info(i)).dim()))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            println!("    {}{}", style(&padded).white(), info_str);
        }
        None => {
            let reason = brief_error(result);
            println!("    {}  {}", style(&padded).white(), style(reason).dim());
        }
    }

    if debug {
        if let Some(duration) = result.check_duration {
            println!(
                "      {} Checked in {}ms via {}",
                style("└─").dim(),
                duration.as_millis(),
                result.method_used,
            );
        }
    }
}

// ── Summary ──────────────────────────────────────────────────────────────────

/// Print the final summary bar with colored counts.
pub fn print_summary(
    total: usize,
    available: usize,
    taken: usize,
    unknown: usize,
    duration: Duration,
) {
    println!(
        "  {}",
        style("────────────────────────────────────────────────────").dim()
    );
    println!(
        "  {} domain{} in {:.1}s  {}  {}  {}  {}  {}  {}",
        style(total).bold(),
        if total == 1 { "" } else { "s" },
        duration.as_secs_f64(),
        style("|").dim(),
        style(format!("{} available", available)).green(),
        style("|").dim(),
        style(format!("{} taken", taken)).red(),
        style("|").dim(),
        style(format!("{} unknown", unknown)).yellow(),
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Format domain info (registrar, dates) into a concise string.
pub fn format_domain_info(info: &DomainInfo) -> String {
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

/// Extract a brief error reason from a DomainResult with unknown status.
fn brief_error(result: &DomainResult) -> &str {
    match &result.error_message {
        Some(msg) => {
            let m = msg.to_lowercase();
            if m.contains("timeout") || m.contains("timed out") {
                "(timeout)"
            } else if m.contains("network") || m.contains("dns") || m.contains("connect") {
                "(network error)"
            } else if m.contains("parse") || m.contains("json") {
                "(parsing error)"
            } else if m.contains("unknown") || m.contains("tld") || m.contains("bootstrap") {
                "(unknown TLD)"
            } else {
                "(error)"
            }
        }
        None => "(unknown status)",
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use domain_check_lib::CheckMethod;

    fn make_result(domain: &str, available: Option<bool>) -> DomainResult {
        DomainResult {
            domain: domain.to_string(),
            available,
            info: None,
            check_duration: None,
            method_used: CheckMethod::Rdap,
            error_message: if available.is_none() {
                Some("timeout".to_string())
            } else {
                None
            },
        }
    }

    fn make_result_with_error(error: &str) -> DomainResult {
        DomainResult {
            domain: "test.com".to_string(),
            available: None,
            info: None,
            check_duration: None,
            method_used: CheckMethod::Unknown,
            error_message: Some(error.to_string()),
        }
    }

    // ── brief_error ─────────────────────────────────────────────────────

    #[test]
    fn test_brief_error_timeout() {
        let r = make_result("a.com", None);
        assert_eq!(brief_error(&r), "(timeout)");
    }

    #[test]
    fn test_brief_error_timed_out() {
        let r = make_result_with_error("request timed out after 5s");
        assert_eq!(brief_error(&r), "(timeout)");
    }

    #[test]
    fn test_brief_error_network() {
        let r = make_result_with_error("dns lookup failed");
        assert_eq!(brief_error(&r), "(network error)");
    }

    #[test]
    fn test_brief_error_connect() {
        let r = make_result_with_error("failed to connect to server");
        assert_eq!(brief_error(&r), "(network error)");
    }

    #[test]
    fn test_brief_error_parsing() {
        let r = make_result_with_error("failed to parse json response");
        assert_eq!(brief_error(&r), "(parsing error)");
    }

    #[test]
    fn test_brief_error_json() {
        let r = make_result_with_error("invalid JSON in response");
        assert_eq!(brief_error(&r), "(parsing error)");
    }

    #[test]
    fn test_brief_error_unknown_tld() {
        let r = make_result_with_error("unknown TLD .xyz123");
        assert_eq!(brief_error(&r), "(unknown TLD)");
    }

    #[test]
    fn test_brief_error_bootstrap() {
        let r = make_result_with_error("bootstrap registry failed");
        assert_eq!(brief_error(&r), "(unknown TLD)");
    }

    #[test]
    fn test_brief_error_generic() {
        let r = make_result_with_error("something unexpected happened");
        assert_eq!(brief_error(&r), "(error)");
    }

    #[test]
    fn test_brief_error_no_message() {
        let r = DomainResult {
            error_message: None,
            ..make_result("a.com", None)
        };
        assert_eq!(brief_error(&r), "(unknown status)");
    }

    #[test]
    fn test_brief_error_case_insensitive() {
        let r = make_result_with_error("TIMEOUT occurred");
        assert_eq!(brief_error(&r), "(timeout)");
    }

    // ── format_domain_info ──────────────────────────────────────────────

    #[test]
    fn test_format_domain_info_all_fields() {
        let info = DomainInfo {
            registrar: Some("GoDaddy".to_string()),
            creation_date: Some("2020-01-01".to_string()),
            expiration_date: Some("2025-01-01".to_string()),
            ..Default::default()
        };
        let formatted = format_domain_info(&info);
        assert!(formatted.contains("Registrar: GoDaddy"));
        assert!(formatted.contains("Created: 2020-01-01"));
        assert!(formatted.contains("Expires: 2025-01-01"));
    }

    #[test]
    fn test_format_domain_info_empty() {
        let info = DomainInfo::default();
        assert_eq!(format_domain_info(&info), "No info available");
    }

    #[test]
    fn test_format_domain_info_registrar_only() {
        let info = DomainInfo {
            registrar: Some("Namecheap".to_string()),
            ..Default::default()
        };
        assert_eq!(format_domain_info(&info), "Registrar: Namecheap");
    }

    #[test]
    fn test_format_domain_info_dates_only() {
        let info = DomainInfo {
            creation_date: Some("2020-01-01".to_string()),
            expiration_date: Some("2025-01-01".to_string()),
            ..Default::default()
        };
        let formatted = format_domain_info(&info);
        assert!(formatted.contains("Created: 2020-01-01"));
        assert!(formatted.contains("Expires: 2025-01-01"));
        assert!(!formatted.contains("Registrar"));
    }

    #[test]
    fn test_format_domain_info_comma_separated() {
        let info = DomainInfo {
            registrar: Some("Reg".to_string()),
            creation_date: Some("2020".to_string()),
            ..Default::default()
        };
        let formatted = format_domain_info(&info);
        assert!(formatted.contains(", "));
    }
}
