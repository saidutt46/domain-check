//! Pretty-mode display logic for domain-check CLI.
//!
//! This module handles all `--pretty` output: colored result lines,
//! grouped batch output, spinner animation, progress counters,
//! headers, and summaries. Uses only the `console` crate (already a dependency).

use console::{pad_str, style, Alignment, Term};
use domain_check_lib::{DomainInfo, DomainResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::{Args, ErrorStats};

// ── Spinner ──────────────────────────────────────────────────────────────────

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// An async braille-dot spinner that writes to stderr so stdout stays clean.
pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Spinner {
    /// Start a new spinner with the given message (e.g. "Checking 8 domains...").
    pub fn start(message: String) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let handle = tokio::spawn(async move {
            let term = Term::stderr();
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

        Self {
            running,
            handle: Some(handle),
        }
    }

    /// Stop the spinner and clear the line.
    pub async fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
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
        style(format!("— Checking {} domain{}", domain_count, if domain_count == 1 { "" } else { "s" })).dim(),
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

// ── Error summary ────────────────────────────────────────────────────────────

/// Print a categorized error summary using colors.
pub fn print_error_summary(error_stats: &ErrorStats, args: &Args) {
    if !error_stats.has_errors() {
        return;
    }

    println!(
        "  {}",
        style("Some domains could not be checked:").yellow()
    );

    let format_list = |domains: &[String], max_show: usize| -> String {
        if domains.len() <= max_show {
            domains.join(", ")
        } else {
            let shown = &domains[..max_show];
            let remaining = domains.len() - max_show;
            format!("{}, ... and {} more", shown.join(", "), remaining)
        }
    };

    if !error_stats.timeouts.is_empty() {
        println!(
            "  {} {} timeout{}: {}",
            style("•").dim(),
            error_stats.timeouts.len(),
            if error_stats.timeouts.len() == 1 { "" } else { "s" },
            format_list(&error_stats.timeouts, 5),
        );
    }
    if !error_stats.network_errors.is_empty() {
        println!(
            "  {} {} network error{}: {}",
            style("•").dim(),
            error_stats.network_errors.len(),
            if error_stats.network_errors.len() == 1 { "" } else { "s" },
            format_list(&error_stats.network_errors, 5),
        );
    }
    if !error_stats.parsing_errors.is_empty() {
        println!(
            "  {} {} parsing error{}: {}",
            style("•").dim(),
            error_stats.parsing_errors.len(),
            if error_stats.parsing_errors.len() == 1 { "" } else { "s" },
            format_list(&error_stats.parsing_errors, 5),
        );
    }
    if !error_stats.unknown_tld_errors.is_empty() {
        println!(
            "  {} {} unknown TLD error{}: {}",
            style("•").dim(),
            error_stats.unknown_tld_errors.len(),
            if error_stats.unknown_tld_errors.len() == 1 { "" } else { "s" },
            format_list(&error_stats.unknown_tld_errors, 5),
        );
    }
    if !error_stats.other_errors.is_empty() {
        println!(
            "  {} {} other error{}: {}",
            style("•").dim(),
            error_stats.other_errors.len(),
            if error_stats.other_errors.len() == 1 { "" } else { "s" },
            format_list(&error_stats.other_errors, 5),
        );
    }

    if args.debug && error_stats.has_errors() {
        println!(
            "  {} {}",
            style("•").dim(),
            style("All errors attempted WHOIS fallback where possible").dim(),
        );
    }
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

    #[test]
    fn test_brief_error_timeout() {
        let r = make_result("a.com", None);
        assert_eq!(brief_error(&r), "(timeout)");
    }

    #[test]
    fn test_brief_error_network() {
        let r = DomainResult {
            error_message: Some("dns lookup failed".to_string()),
            ..make_result("a.com", None)
        };
        assert_eq!(brief_error(&r), "(network error)");
    }

    #[test]
    fn test_brief_error_unknown_status() {
        let r = DomainResult {
            error_message: None,
            ..make_result("a.com", None)
        };
        assert_eq!(brief_error(&r), "(unknown status)");
    }

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
    fn test_format_domain_info_partial() {
        let info = DomainInfo {
            registrar: Some("Namecheap".to_string()),
            ..Default::default()
        };
        assert_eq!(format_domain_info(&info), "Registrar: Namecheap");
    }
}
