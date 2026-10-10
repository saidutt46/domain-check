//! WHOIS protocol implementation for domain availability checking.
//!
//! WHOIS (RFC 3912) is the fallback when RDAP is not available. The client
//! speaks the protocol directly over TCP port 43, so it needs no system
//! `whois` binary and behaves the same on every platform.
//!
//! WHOIS replies are free text and every registry formats them differently,
//! so [`classify`] reads them as `key: value` fields plus a small set of
//! "not found" phrases, and answers [`Verdict::Unknown`] rather than guess.

use crate::error::DomainCheckError;
use crate::types::{CheckMethod, DomainResult};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Well-known WHOIS port (RFC 3912).
const WHOIS_PORT: u16 = 43;

/// Replies larger than this are cut off; real ones are a few KB.
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

/// IANA's WHOIS server, which refers each TLD to its registry's server.
const IANA_WHOIS_SERVER: &str = "whois.iana.org";

/// Message for replies that are neither clearly taken nor clearly free.
/// The checker matches on this text to report UNKNOWN.
const UNDETERMINED: &str = "Unable to determine domain status from WHOIS response";

/// WHOIS client for checking domain availability.
#[derive(Clone)]
pub struct WhoisClient {
    /// Timeout for one WHOIS query (connect, send, and read)
    timeout: Duration,
}

impl WhoisClient {
    /// Create a new WHOIS client with default settings.
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }

    /// Create a new WHOIS client with custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Check domain availability, discovering the registry's WHOIS server
    /// through IANA.
    pub async fn check_domain(&self, domain: &str) -> Result<DomainResult, DomainCheckError> {
        let tld = crate::protocols::registry::extract_tld(domain)?;
        match crate::protocols::registry::get_whois_server(&tld).await {
            Some(server) => self.check_domain_with_server(domain, &server).await,
            None => Err(DomainCheckError::whois(
                domain,
                format!("No WHOIS server is known for .{tld}"),
            )),
        }
    }

    /// Check domain availability against a specific WHOIS server.
    pub async fn check_domain_with_server(
        &self,
        domain: &str,
        server: &str,
    ) -> Result<DomainResult, DomainCheckError> {
        let start_time = Instant::now();
        let query = query_for(server, domain);

        let mut verdict = classify(&self.query(server, &query, domain).await?, domain);
        if verdict == Verdict::RateLimited {
            tokio::time::sleep(Duration::from_secs(1)).await;
            verdict = classify(&self.query(server, &query, domain).await?, domain);
        }

        let available = match verdict {
            Verdict::Taken => false,
            Verdict::Available => true,
            Verdict::RateLimited => {
                return Err(DomainCheckError::whois(
                    domain,
                    format!("{server} rate limited the query"),
                ))
            }
            Verdict::Unknown(reason) => return Err(DomainCheckError::whois(domain, reason)),
        };

        Ok(DomainResult {
            domain: domain.to_string(),
            available: Some(available),
            info: None,
            check_duration: Some(start_time.elapsed()),
            method_used: CheckMethod::Whois,
            error_message: None,
            for_sale: None,
        })
    }

    async fn query(
        &self,
        server: &str,
        query: &str,
        domain: &str,
    ) -> Result<String, DomainCheckError> {
        match tokio::time::timeout(self.timeout, raw_query(server, query)).await {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => Err(DomainCheckError::whois(
                domain,
                format!("WHOIS query to {server} failed: {e}"),
            )),
            Err(_) => Err(DomainCheckError::timeout("WHOIS query", self.timeout)),
        }
    }
}

impl Default for WhoisClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Send one RFC 3912 query and read the reply until the server closes.
async fn raw_query(server: &str, query: &str) -> std::io::Result<String> {
    // Connect to every address at once and keep the first that answers:
    // some registries publish an IPv4 or IPv6 address that never responds.
    let attempts: Vec<_> = tokio::net::lookup_host((server, WHOIS_PORT))
        .await?
        .map(|addr| Box::pin(TcpStream::connect(addr)))
        .collect();
    if attempts.is_empty() {
        return Err(std::io::Error::other(format!("{server} has no addresses")));
    }
    let (mut stream, _) = futures_util::future::select_ok(attempts).await?;
    stream.write_all(format!("{query}\r\n").as_bytes()).await?;
    let mut reply = Vec::new();
    stream
        .take(MAX_RESPONSE_BYTES)
        .read_to_end(&mut reply)
        .await?;
    // Some registries still answer in Latin-1; only field values are affected.
    Ok(String::from_utf8_lossy(&reply).into_owned())
}

/// Registry-specific query syntax.
fn query_for(server: &str, domain: &str) -> String {
    if server.eq_ignore_ascii_case("whois.jprs.jp") {
        // Without "/e" JPRS answers with Japanese field names.
        format!("{domain}/e")
    } else {
        domain.to_string()
    }
}

/// Discover the authoritative WHOIS server for a TLD via IANA referral.
///
/// Returns the server hostname (e.g. "whois.nic.it"), or None if IANA lists
/// none or the query failed.
pub async fn discover_whois_server(tld: &str) -> Option<String> {
    let reply = tokio::time::timeout(Duration::from_secs(10), raw_query(IANA_WHOIS_SERVER, tld))
        .await
        .ok()?
        .ok()?;
    parse_iana_refer_response(&reply)
}

/// Parse an IANA WHOIS response for the authoritative WHOIS server.
///
/// The IANA WHOIS response may use either `refer:` or `whois:` to indicate
/// the authoritative WHOIS server for a TLD. We check both fields, preferring
/// `refer:` when present.
///
/// ```text
/// whois:        whois.verisign-grs.com
/// refer:        whois.verisign-grs.com
/// ```
fn parse_iana_refer_response(response: &str) -> Option<String> {
    let mut whois_server = None;

    for line in response.lines() {
        let line_trimmed = line.trim();
        if let Some(server) = line_trimmed.strip_prefix("refer:") {
            let server = server.trim();
            if !server.is_empty() {
                // `refer:` is the canonical field — return immediately
                return Some(server.to_string());
            }
        } else if let Some(server) = line_trimmed.strip_prefix("whois:") {
            let server = server.trim();
            if !server.is_empty() {
                whois_server = Some(server.to_string());
            }
        }
    }

    whois_server
}

/// What a WHOIS reply says about a domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Taken,
    Available,
    RateLimited,
    Unknown(&'static str),
}

// Kinds of registration data. A reply with two of them (or one plus the
// domain name echoed back) describes a registered domain.
const CREATED: u8 = 1;
const EXPIRES: u8 = 1 << 1;
const UPDATED: u8 = 1 << 2;
const REGISTRAR: u8 = 1 << 3;
const REGISTRANT: u8 = 1 << 4;
const NAME_SERVERS: u8 = 1 << 5;
const STATUS: u8 = 1 << 6;

/// Phrases registries use for an unregistered name, collected from real
/// replies (see `tests/fixtures/whois`). Only consulted when the reply holds
/// no registration data, so a disclaimer can't turn a taken domain free.
const FREE_PHRASES: &[&str] = &[
    "no match",
    "not found",
    "no matching record",
    "no matching entry",
    "no entries found",
    "no data found",
    "no data was found",
    "no information available about",
    "nothing found",
    "no such domain",
    "does not exist",
    "not registered",
    "has not been registered",
    "is available for registration",
    "domain available",
    "object_not_found",
    "no object found",
];

const RATE_LIMIT_PHRASES: &[&str] = &[
    "rate limit",
    "query rate",
    "too many",
    "quota exceeded",
    "limit exceeded",
    "queries exceeded",
    "number of allowed queries",
    "try again later",
    "throttled",
    "rate-limited",
];

const REFUSED_PHRASES: &[&str] = &[
    "not permitted",
    "access denied",
    "not authorised",
    "not authorized",
    "refused",
];

/// Decide whether a WHOIS reply describes a registered or a free domain.
///
/// Registration data wins over everything else; "free" needs an explicit
/// phrase or status. Anything else is unknown: an empty, refused, or
/// unfamiliar reply must never read as AVAILABLE.
pub(crate) fn classify(response: &str, domain: &str) -> Verdict {
    let text = response.to_lowercase().replace('\t', " ");
    let domain = domain.trim_end_matches('.').to_lowercase();

    let mut evidence = 0u8;
    let mut echoed = false;
    let mut free_status = false;
    for line in text.lines().map(str::trim) {
        if is_comment(line) {
            continue;
        }
        let Some((key, value)) = split_field(line) else {
            continue;
        };
        if is_status_key(key) && is_free_value(value) {
            free_status = true;
        } else if is_domain_key(key) {
            echoed |= value
                .split_whitespace()
                .next()
                .map(|v| v.trim_end_matches('.'))
                == Some(domain.as_str());
        } else {
            evidence |= category(key, value);
        }
    }

    if evidence.count_ones() >= 2 || (echoed && evidence != 0) {
        return Verdict::Taken;
    }
    let free_phrase = format!("{domain} is free");
    if free_status
        || text
            .lines()
            .any(|l| l.contains(&free_phrase) || FREE_PHRASES.iter().any(|p| l.contains(p)))
    {
        return Verdict::Available;
    }
    if RATE_LIMIT_PHRASES.iter().any(|p| text.contains(p)) {
        return Verdict::RateLimited;
    }
    if REFUSED_PHRASES.iter().any(|p| text.contains(p)) {
        return Verdict::Unknown("WHOIS server refused the query");
    }
    if text.trim().is_empty() {
        return Verdict::Unknown("WHOIS server sent an empty reply");
    }
    Verdict::Unknown(UNDETERMINED)
}

fn is_comment(line: &str) -> bool {
    line.starts_with('%') || line.starts_with('#') || line.starts_with(">>>")
}

/// Split `key: value` or JPRS-style `[key] value`.
fn split_field(line: &str) -> Option<(&str, &str)> {
    let (key, value) = if let Some(rest) = line.strip_prefix('[') {
        rest.split_once(']')?
    } else {
        line.split_once(':')?
    };
    let key = key.trim();
    if key.is_empty() || key.len() > 40 || key.starts_with("http") {
        return None;
    }
    Some((key, value.trim()))
}

fn is_domain_key(key: &str) -> bool {
    matches!(key, "domain" | "domain name" | "domainname" | "domain-name")
}

fn is_status_key(key: &str) -> bool {
    key == "status" || key.ends_with(" status") || key == "domaintype"
}

fn is_free_value(value: &str) -> bool {
    value.starts_with("available")
        || value.starts_with("free")
        || value.starts_with("not registered")
        || value.starts_with("no object found")
}

/// Map one field to the kind of registration data it carries, if any.
fn category(key: &str, value: &str) -> u8 {
    let has_value = !value.is_empty();
    // Headers whose data follows on the next lines (e.g. `.be`, `.bg`).
    let header = |k: &str| key == k;

    if is_status_key(key) && has_value {
        STATUS
    } else if (key.contains("creat")
        || key == "registered"
        || key.contains("registration date")
        || key.contains("registration time"))
        && has_value
    {
        CREATED
    } else if (key.contains("expir") || key.contains("paid-till") || key.contains("renewal"))
        && has_value
    {
        EXPIRES
    } else if (key.contains("updat") || key.contains("changed") || key.contains("modified"))
        && !key.contains("whois database")
        && has_value
    {
        UPDATED
    } else if key.starts_with("registrar") || key.starts_with("sponsoring registrar") {
        if has_value || header("registrar") {
            REGISTRAR
        } else {
            0
        }
    } else if key.contains("registrant") || key == "holder" || key.starts_with("owner") {
        if has_value || header("registrant") {
            REGISTRANT
        } else {
            0
        }
    } else if key.contains("nserver") || key.contains("name server") || key.contains("nameserver") {
        NAME_SERVERS
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── WhoisClient creation ────────────────────────────────────────────

    #[test]
    fn test_whois_client_new() {
        let client = WhoisClient::new();
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_whois_client_with_timeout() {
        let client = WhoisClient::with_timeout(Duration::from_secs(10));
        assert_eq!(client.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_whois_client_default() {
        let client = WhoisClient::default();
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    // ── classify: real replies from each registry ──────────────────────
    //
    // Captured 2026-10-09 with `google.<tld>` (taken) and
    // `zq7x9k2m4ptest.<tld>` (free); personal contact fields redacted.

    macro_rules! fixture {
        ($file:literal) => {
            include_str!(concat!("../../tests/fixtures/whois/", $file))
        };
    }

    fn taken_domain(tld: &str) -> String {
        match tld {
            "uk" => "google.co.uk".into(),
            "il" => "google.co.il".into(),
            "nz" => "stuff.co.nz".into(),
            _ => format!("google.{tld}"),
        }
    }

    fn free_domain(tld: &str) -> String {
        match tld {
            "il" => "zq7x9k2m4ptest.co.il".into(),
            _ => format!("zq7x9k2m4ptest.{tld}"),
        }
    }

    const FIXTURES: &[(&str, &str, &str)] = &[
        ("ai", fixture!("ai.taken.txt"), fixture!("ai.free.txt")),
        ("at", fixture!("at.taken.txt"), fixture!("at.free.txt")),
        ("be", fixture!("be.taken.txt"), fixture!("be.free.txt")),
        ("bg", fixture!("bg.taken.txt"), fixture!("bg.free.txt")),
        ("cl", fixture!("cl.taken.txt"), fixture!("cl.free.txt")),
        ("cn", fixture!("cn.taken.txt"), fixture!("cn.free.txt")),
        ("co", fixture!("co.taken.txt"), fixture!("co.free.txt")),
        ("com", fixture!("com.taken.txt"), fixture!("com.free.txt")),
        ("cz", fixture!("cz.taken.txt"), fixture!("cz.free.txt")),
        ("de", fixture!("de.taken.txt"), fixture!("de.free.txt")),
        ("dk", fixture!("dk.taken.txt"), fixture!("dk.free.txt")),
        ("eu", fixture!("eu.taken.txt"), fixture!("eu.free.txt")),
        ("fi", fixture!("fi.taken.txt"), fixture!("fi.free.txt")),
        ("fr", fixture!("fr.taken.txt"), fixture!("fr.free.txt")),
        ("gg", fixture!("gg.taken.txt"), fixture!("gg.free.txt")),
        ("hk", fixture!("hk.taken.txt"), fixture!("hk.free.txt")),
        ("hr", fixture!("hr.taken.txt"), fixture!("hr.free.txt")),
        ("hu", fixture!("hu.taken.txt"), fixture!("hu.free.txt")),
        ("ie", fixture!("ie.taken.txt"), fixture!("ie.free.txt")),
        ("il", fixture!("il.taken.txt"), fixture!("il.free.txt")),
        ("io", fixture!("io.taken.txt"), fixture!("io.free.txt")),
        ("it", fixture!("it.taken.txt"), fixture!("it.free.txt")),
        ("je", fixture!("je.taken.txt"), fixture!("je.free.txt")),
        ("jp", fixture!("jp.taken.txt"), fixture!("jp.free.txt")),
        ("lu", fixture!("lu.taken.txt"), fixture!("lu.free.txt")),
        ("me", fixture!("me.taken.txt"), fixture!("me.free.txt")),
        ("mx", fixture!("mx.taken.txt"), fixture!("mx.free.txt")),
        ("my", fixture!("my.taken.txt"), fixture!("my.free.txt")),
        ("net", fixture!("net.taken.txt"), fixture!("net.free.txt")),
        ("nl", fixture!("nl.taken.txt"), fixture!("nl.free.txt")),
        ("no", fixture!("no.taken.txt"), fixture!("no.free.txt")),
        ("nz", fixture!("nz.taken.txt"), fixture!("nz.free.txt")),
        ("org", fixture!("org.taken.txt"), fixture!("org.free.txt")),
        ("pl", fixture!("pl.taken.txt"), fixture!("pl.free.txt")),
        ("pt", fixture!("pt.taken.txt"), fixture!("pt.free.txt")),
        ("ro", fixture!("ro.taken.txt"), fixture!("ro.free.txt")),
        ("ru", fixture!("ru.taken.txt"), fixture!("ru.free.txt")),
        ("se", fixture!("se.taken.txt"), fixture!("se.free.txt")),
        ("sk", fixture!("sk.taken.txt"), fixture!("sk.free.txt")),
        ("tr", fixture!("tr.taken.txt"), fixture!("tr.free.txt")),
        ("uk", fixture!("uk.taken.txt"), fixture!("uk.free.txt")),
        ("us", fixture!("us.taken.txt"), fixture!("us.free.txt")),
        ("xyz", fixture!("xyz.taken.txt"), fixture!("xyz.free.txt")),
    ];

    #[test]
    fn real_taken_replies_are_taken() {
        let wrong: Vec<_> = FIXTURES
            .iter()
            .filter_map(|(tld, taken, _)| {
                let v = classify(taken, &taken_domain(tld));
                (v != Verdict::Taken).then(|| format!("{tld}: {v:?}"))
            })
            .collect();
        assert!(wrong.is_empty(), "misread taken replies: {wrong:?}");
    }

    #[test]
    fn real_free_replies_are_available() {
        let wrong: Vec<_> = FIXTURES
            .iter()
            .filter_map(|(tld, _, free)| {
                let v = classify(free, &free_domain(tld));
                (v != Verdict::Available).then(|| format!("{tld}: {v:?}"))
            })
            .collect();
        assert!(wrong.is_empty(), "misread free replies: {wrong:?}");
    }

    #[test]
    fn taken_reply_is_never_available_for_another_name() {
        // A taken reply must not read as free just because the queried name
        // differs (e.g. the registry normalised it).
        for (tld, taken, _) in FIXTURES {
            assert_ne!(
                classify(taken, "other-name.example"),
                Verdict::Available,
                "{tld}"
            );
        }
    }

    #[test]
    fn refused_replies_are_unknown() {
        // .ch and .li only answer on their website.
        for reply in [fixture!("ch.taken.txt"), fixture!("li.free.txt")] {
            assert_eq!(
                classify(reply, "google.ch"),
                Verdict::Unknown("WHOIS server refused the query")
            );
        }
    }

    // ── classify: edge cases ────────────────────────────────────────────

    #[test]
    fn empty_reply_is_unknown_not_available() {
        // Used to be AVAILABLE (any reply under 50 characters counted as free).
        assert!(matches!(classify("", "x.com"), Verdict::Unknown(_)));
        assert!(matches!(classify("  \r\n", "x.com"), Verdict::Unknown(_)));
        assert!(matches!(
            classify("Some short text", "x.com"),
            Verdict::Unknown(_)
        ));
    }

    #[test]
    fn rate_limit_in_comment_is_detected() {
        let lu = "% WHOIS zq7x9k2m4ptest.lu\n%% Maximum query rate reached\n";
        assert_eq!(classify(lu, "zq7x9k2m4ptest.lu"), Verdict::RateLimited);
        assert_eq!(
            classify("Rate limit exceeded. Try again later.", "x.com"),
            Verdict::RateLimited
        );
    }

    #[test]
    fn disclaimer_phrases_do_not_override_registration_data() {
        let reply = "Domain Name: x.com\nRegistrar: Example\nCreation Date: 2020-01-01\n\
                     % If a domain is not found, it may be available.";
        assert_eq!(classify(reply, "x.com"), Verdict::Taken);
    }

    #[test]
    fn not_available_status_is_taken() {
        let be = "Domain:\tx.be\nStatus:\tNOT AVAILABLE\n";
        assert_eq!(classify(be, "x.be"), Verdict::Taken);
    }

    #[test]
    fn free_status_with_echo_is_available() {
        assert_eq!(
            classify("Domain: x.de\nStatus: free\n", "x.de"),
            Verdict::Available
        );
        assert_eq!(
            classify(
                "Domain:             x.it\nStatus:             AVAILABLE\n",
                "x.it"
            ),
            Verdict::Available
        );
    }

    #[test]
    fn single_field_without_echo_is_unknown() {
        let reply = "Registrar: SomeRegistrar\nSome other random text that is long enough";
        assert_eq!(classify(reply, "x.com"), Verdict::Unknown(UNDETERMINED));
    }

    #[test]
    fn whois_database_timestamp_is_not_evidence() {
        let reply = "No match for \"X.COM\".\n>>> Last update of whois database: 2026-10-10 <<<";
        assert_eq!(classify(reply, "x.com"), Verdict::Available);
    }

    #[test]
    fn jprs_query_uses_english_suffix() {
        assert_eq!(query_for("whois.jprs.jp", "google.jp"), "google.jp/e");
        assert_eq!(query_for("whois.nic.it", "google.it"), "google.it");
    }

    // ── parse_iana_refer_response ───────────────────────────────────────

    #[test]
    fn test_iana_refer_standard() {
        let response =
            "% IANA WHOIS server\n\nrefer:        whois.verisign-grs.com\n\ndomain:       COM\n";
        assert_eq!(
            parse_iana_refer_response(response),
            Some("whois.verisign-grs.com".to_string())
        );
    }

    #[test]
    fn test_iana_refer_none() {
        let response = "% IANA WHOIS server\ndomain: TEST\nstatus: ACTIVE\n";
        assert_eq!(parse_iana_refer_response(response), None);
    }

    #[test]
    fn test_iana_refer_empty_value() {
        let response = "refer:        \ndomain: COM\n";
        assert_eq!(parse_iana_refer_response(response), None);
    }

    #[test]
    fn test_iana_whois_field_fallback() {
        let response = "whois:        whois.verisign-grs.com\ndomain: COM\n";
        assert_eq!(
            parse_iana_refer_response(response),
            Some("whois.verisign-grs.com".to_string())
        );
    }

    #[test]
    fn test_iana_refer_takes_precedence_over_whois() {
        let response =
            "whois:        whois.old-server.com\nrefer:        whois.correct-server.com\n";
        assert_eq!(
            parse_iana_refer_response(response),
            Some("whois.correct-server.com".to_string())
        );
    }

    #[test]
    fn test_iana_empty_whois_field() {
        let response = "whois:        \ndomain: COM\n";
        assert_eq!(parse_iana_refer_response(response), None);
    }

    #[test]
    fn test_iana_empty_response() {
        assert_eq!(parse_iana_refer_response(""), None);
    }

    // ── Network-dependent tests ─────────────────────────────────────────

    #[tokio::test]
    async fn live_whois_over_tcp() {
        let client = WhoisClient::new();
        match client
            .check_domain_with_server("google.com", "whois.verisign-grs.com")
            .await
        {
            Ok(result) => assert_eq!(result.available, Some(false)),
            // Port 43 may be blocked on some networks; don't fail on that.
            Err(e) => eprintln!("skipped: WHOIS over TCP unavailable: {e}"),
        }
    }

    #[tokio::test]
    async fn live_iana_discovery() {
        if let Some(server) = discover_whois_server("it").await {
            assert_eq!(server, "whois.nic.it");
        }
    }
}
