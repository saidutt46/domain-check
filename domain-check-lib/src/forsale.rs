//! RFC 10023 `_for-sale` records.
//!
//! A domain holder can publish `_for-sale.<domain> TXT "v=FORSALE1;..."` to
//! signal that a registered domain is available for purchase. The parser here
//! is pure; the DNS lookup is compiled only with the `forsale` feature.

use crate::types::{CheckConfig, DomainResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Leaf node name reserved by RFC 10023.
pub const NODE_NAME: &str = "_for-sale";

const VERSION_TAG: &str = "v=FORSALE1;";
/// Maximum octets for fcod/ftxt/fval content values (RFC 10023 §2.1).
const MAX_VALUE_LEN: usize = 239;

/// Parsed `_for-sale` RRset. Present means the domain is for sale,
/// even when every list is empty (RFC 10023 §2.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ForSaleInfo {
    /// `furi=` contact URIs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uris: Vec<ForSaleUri>,
    /// `fval=` asking prices. Indicative only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prices: Vec<ForSalePrice>,
    /// `ftxt=` free text, sanitized.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<String>,
    /// `fcod=` codes, meaningful only to cooperating parties.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
}

impl ForSaleInfo {
    /// True when a valid version tag was found but no usable content.
    pub fn is_empty(&self) -> bool {
        self.uris.is_empty()
            && self.prices.is_empty()
            && self.texts.is_empty()
            && self.codes.is_empty()
    }

    /// First URI with a recommended scheme (http, https, mailto, tel).
    pub fn first_trusted_uri(&self) -> Option<&ForSaleUri> {
        self.uris.iter().find(|u| u.trusted)
    }
}

/// A `furi=` value. Untrusted schemes are kept but must not be shown as links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ForSaleUri {
    pub value: String,
    /// Lowercased scheme, e.g. "https".
    pub scheme: String,
    /// Scheme is one RFC 10023 recommends: http, https, mailto, tel.
    pub trusted: bool,
}

/// A `fval=` asking price. The amount stays a string to avoid rounding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ForSalePrice {
    pub currency: String,
    pub amount: String,
}

impl std::fmt::Display for ForSalePrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.currency, self.amount)
    }
}

/// Absolute DNS name to query for `domain`, or `None` where RFC 10023
/// does not apply (empty names and the `.arpa` infrastructure TLD, §2.6).
pub fn record_name(domain: &str) -> Option<String> {
    let domain = domain.trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() || domain == "arpa" || domain.ends_with(".arpa") {
        return None;
    }
    // Trailing dot: never let resolver search domains be appended.
    Some(format!("{NODE_NAME}.{domain}."))
}

/// Parse the TXT RRset at `_for-sale.<domain>`.
///
/// Each item is one TXT record given as its character-strings. Returns `None`
/// when no record carries a valid version tag, meaning the domain is not for sale.
pub fn parse_txt_records(records: &[Vec<Vec<u8>>]) -> Option<ForSaleInfo> {
    let mut info = ForSaleInfo::default();
    let mut valid = false;
    let mut seen = HashSet::new();

    for record in records {
        // RDATA must be a single character-string (§2.4).
        let [single] = record.as_slice() else {
            continue;
        };
        let text = String::from_utf8_lossy(single);
        let Some(content) = text.strip_prefix(VERSION_TAG) else {
            continue;
        };
        valid = true;

        // Spaces after the version tag are tolerated (§3.6).
        let content = content.trim_start_matches(' ');
        if !seen.insert(content.to_string()) {
            continue;
        }

        // One pair per record and the value runs to the end (§3.5), so
        // never split on ';' — URIs may contain one.
        let Some((tag, value)) = content.split_once('=') else {
            continue;
        };
        match tag {
            "ftxt" => push_text(&mut info.texts, value),
            "fcod" => push_text(&mut info.codes, value),
            "fval" => info.prices.extend(parse_price(value)),
            "furi" => info.uris.extend(parse_uri(value)),
            // Future tags (§2.2.5): the record still marks the domain for sale.
            _ => {}
        }
    }

    if !valid {
        return None;
    }

    // An RRset has no inherent order; sort for stable output.
    info.texts.sort();
    info.codes.sort();
    info.uris.sort_by(|a, b| a.value.cmp(&b.value));
    info.prices
        .sort_by(|a, b| (&a.currency, &a.amount).cmp(&(&b.currency, &b.amount)));
    Some(info)
}

fn push_text(list: &mut Vec<String>, value: &str) {
    if value.is_empty() || value.len() > MAX_VALUE_LEN {
        return;
    }
    let clean = sanitize(value);
    if !clean.is_empty() {
        list.push(clean);
    }
}

fn parse_price(value: &str) -> Option<ForSalePrice> {
    let value = value.trim();
    if value.len() < 2 || value.len() > MAX_VALUE_LEN {
        return None;
    }
    let split = value.find(|c: char| !c.is_ascii_uppercase())?;
    let (currency, amount) = value.split_at(split);
    let (int, frac) = match amount.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (amount, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if currency.is_empty() || !digits(int) || frac.is_some_and(|f| !digits(f)) {
        return None;
    }
    Some(ForSalePrice {
        currency: currency.to_string(),
        amount: amount.to_string(),
    })
}

fn parse_uri(value: &str) -> Option<ForSaleUri> {
    let value = sanitize(value);
    // Spaces must be percent-encoded (§2.2.3).
    if value.contains(' ') {
        return None;
    }
    let (scheme, rest) = value.split_once(':')?;
    let mut chars = scheme.chars();
    let valid_scheme = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !valid_scheme || rest.is_empty() {
        return None;
    }
    let scheme = scheme.to_ascii_lowercase();
    let trusted = match scheme.as_str() {
        "http" | "https" => rest
            .strip_prefix("//")
            .is_some_and(|r| !r.is_empty() && !r.starts_with('/')),
        "mailto" | "tel" => true,
        _ => false,
    };
    Some(ForSaleUri {
        value,
        scheme,
        trusted,
    })
}

/// Make holder-supplied text safe to print: no terminal escapes, no bidi
/// reordering, and no invisible characters that could hide text from a human
/// reader while an LLM still sees it (RFC 10023 §3.6, §4).
pub(crate) fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter_map(|c| match c {
            '\t' | '\n' | '\r' | '\u{2028}' | '\u{2029}' => Some(' '),
            c if c.is_control() || is_bidi_control(c) || is_invisible(c) => None,
            c => Some(c),
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Invisible format characters. Joiners (U+200C/U+200D) are kept because
/// emoji sequences and some scripts depend on them.
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}' | '\u{180E}' | '\u{200B}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' | '\u{E0000}'..='\u{E007F}'
    )
}

fn is_bidi_control(c: char) -> bool {
    matches!(
        c,
        '\u{200E}' | '\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// Upper bound for one `_for-sale` lookup; also capped by the check timeout.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(3);

/// Runs `_for-sale` lookups for taken results. Inactive when not requested,
/// when built without the `forsale` feature, or when the system resolver
/// configuration cannot be read.
#[derive(Clone)]
pub(crate) struct ForSaleLookup {
    #[cfg(feature = "forsale")]
    resolver: Option<hickory_resolver::TokioResolver>,
    #[cfg_attr(not(feature = "forsale"), allow(dead_code))]
    timeout: Duration,
}

impl ForSaleLookup {
    pub(crate) fn new(config: &CheckConfig) -> Self {
        let timeout = config.timeout.min(LOOKUP_TIMEOUT);
        #[cfg(feature = "forsale")]
        {
            let resolver = config
                .check_for_sale
                .then(|| {
                    hickory_resolver::TokioResolver::builder_tokio()
                        .ok()?
                        .build()
                        .ok()
                })
                .flatten();
            Self { resolver, timeout }
        }
        #[cfg(not(feature = "forsale"))]
        {
            let _ = config.check_for_sale;
            Self { timeout }
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        #[cfg(feature = "forsale")]
        {
            self.resolver.is_some()
        }
        #[cfg(not(feature = "forsale"))]
        {
            false
        }
    }

    /// Attach for-sale info to a taken result. Never fails and never
    /// changes the availability verdict.
    pub(crate) async fn annotate(&self, result: &mut DomainResult) {
        #[cfg(feature = "forsale")]
        if let (Some(resolver), Some(false)) = (&self.resolver, result.available) {
            result.for_sale = lookup(resolver, &result.domain, self.timeout).await;
        }
        #[cfg(not(feature = "forsale"))]
        let _ = result;
    }
}

#[cfg(all(test, feature = "forsale"))]
impl ForSaleLookup {
    /// Test-only: a lookup that queries a single, specific nameserver.
    fn with_nameserver(ip: std::net::IpAddr, timeout: Duration) -> Self {
        use hickory_resolver::config::{NameServerConfig, ResolverConfig};
        use hickory_resolver::net::runtime::TokioRuntimeProvider;

        let config = ResolverConfig::from_name_servers(vec![NameServerConfig::udp(ip)]);
        let resolver = hickory_resolver::TokioResolver::builder_with_config(
            config,
            TokioRuntimeProvider::default(),
        )
        .build()
        .ok();
        Self { resolver, timeout }
    }
}

#[cfg(feature = "forsale")]
async fn lookup(
    resolver: &hickory_resolver::TokioResolver,
    domain: &str,
    timeout: Duration,
) -> Option<ForSaleInfo> {
    use hickory_resolver::proto::rr::RData;

    let name = record_name(domain)?;
    let answer = tokio::time::timeout(timeout, resolver.txt_lookup(name.as_str()))
        .await
        .ok()?
        .ok()?;
    let records: Vec<Vec<Vec<u8>>> = answer
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::TXT(txt) => Some(txt.txt_data.iter().map(|s| s.to_vec()).collect()),
            _ => None,
        })
        .collect();
    parse_txt_records(&records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(s: &str) -> Vec<Vec<u8>> {
        vec![s.as_bytes().to_vec()]
    }
    fn parse(records: &[&str]) -> Option<ForSaleInfo> {
        let rs: Vec<Vec<Vec<u8>>> = records.iter().map(|s| rec(s)).collect();
        parse_txt_records(&rs)
    }

    // ── version tag (§2.1) — variants published at _for-sale.forsale.testdns.nl ──
    #[test]
    fn version_tag_variants() {
        assert!(parse(&["v=FORSALE1;"]).is_some());
        assert!(parse(&["v=FORSALE1"]).is_none());
        assert!(parse(&["v=FORSALE"]).is_none());
        assert!(parse(&["v=FORSALE;"]).is_none());
        assert!(parse(&["v=FORSALE0;"]).is_none());
        assert!(parse(&["V=FORSALE1;"]).is_none(), "case-sensitive");
        assert!(parse(&["v=FORSALE10;"]).is_none());
        assert!(parse(&[" v=FORSALE1;"]).is_none(), "must start the record");
    }

    #[test]
    fn empty_rrset_is_not_for_sale() {
        assert!(parse(&[]).is_none());
        assert!(parse(&["v=spf1 -all", "I am for sale"]).is_none());
    }

    #[test]
    fn version_only_is_for_sale_and_empty() {
        let info = parse(&["v=FORSALE1;"]).unwrap();
        assert!(info.is_empty());
    }

    #[test]
    fn invalid_or_unknown_content_still_for_sale() {
        for r in [
            "v=FORSALE1;fcod=",
            "v=FORSALE1;foo=bar",
            "v=FORSALE1;garbage",
            "v=FORSALE1;fval=cheap",
        ] {
            let info = parse(&[r]).unwrap_or_else(|| panic!("{r} should be for sale"));
            assert!(info.is_empty(), "{r}");
        }
    }

    #[test]
    fn spaces_after_version_tolerated() {
        let info = parse(&["v=FORSALE1;   ftxt=hello"]).unwrap();
        assert_eq!(info.texts, vec!["hello"]);
    }

    #[test]
    fn records_without_version_are_ignored_alongside_valid() {
        let info = parse(&[
            "This line is not part of the ForSale record.",
            "v=FORSALE1;ftxt=hi",
        ])
        .unwrap();
        assert_eq!(info.texts, vec!["hi"]);
    }

    // ── RRset rules (§2.4) ──
    #[test]
    fn multi_string_record_is_invalid() {
        let split = vec![b"v=FORSALE1;".to_vec(), b"ftxt=foo".to_vec()];
        assert!(parse_txt_records(&[split]).is_none());
    }

    #[test]
    fn duplicates_are_collapsed_and_output_sorted() {
        let info = parse(&[
            "v=FORSALE1;ftxt=b",
            "v=FORSALE1;ftxt=a",
            "v=FORSALE1;ftxt=b",
            "v=FORSALE1;fval=USD5",
            "v=FORSALE1;fval=EUR9",
        ])
        .unwrap();
        assert_eq!(info.texts, vec!["a", "b"]);
        assert_eq!(info.prices[0].currency, "EUR");
        assert_eq!(info.prices[1].currency, "USD");
    }

    // ── tag parsing (§2.2, §3.5) ──
    #[test]
    fn all_tags_parse() {
        let info = parse(&[
            "v=FORSALE1;furi=https://example.nl/for-sale.txt",
            "v=FORSALE1;fval=EUR100000000",
            "v=FORSALE1;ftxt=See the URL for important information!",
            "v=FORSALE1;fcod=NLFS-NGYyYjEyZWY",
        ])
        .unwrap();
        assert_eq!(info.uris[0].value, "https://example.nl/for-sale.txt");
        assert!(info.uris[0].trusted);
        assert_eq!(info.prices[0].to_string(), "EUR 100000000");
        assert_eq!(info.texts[0], "See the URL for important information!");
        assert_eq!(info.codes[0], "NLFS-NGYyYjEyZWY");
    }

    #[test]
    fn value_runs_to_end_of_record() {
        let info = parse(&["v=FORSALE1;fcod=TRIP-confusing;ftxt=dont_do_this"]).unwrap();
        assert_eq!(info.codes, vec!["TRIP-confusing;ftxt=dont_do_this"]);
        assert!(info.texts.is_empty());
        let info = parse(&["v=FORSALE1;furi=https://x.example/a;b=c?d=e"]).unwrap();
        assert_eq!(info.uris[0].value, "https://x.example/a;b=c?d=e");
    }

    #[test]
    fn unicode_text_kept() {
        let info = parse(&["v=FORSALE1;ftxt=🤓"]).unwrap();
        assert_eq!(info.texts, vec!["🤓"]);
    }

    #[test]
    fn invalid_utf8_replaced() {
        let bad = vec![[b"v=FORSALE1;ftxt=a".as_slice(), &[0xFF], b"b"].concat()];
        let info = parse_txt_records(&[bad]).unwrap();
        assert_eq!(info.texts, vec!["a\u{FFFD}b"]);
    }

    #[test]
    fn value_length_limits() {
        let ok = format!("v=FORSALE1;ftxt={}", "a".repeat(239));
        let too_long = format!("v=FORSALE1;ftxt={}", "a".repeat(240));
        assert_eq!(parse(&[&ok]).unwrap().texts.len(), 1);
        assert!(parse(&[&too_long]).unwrap().texts.is_empty());
    }

    // ── fval (§2.2.4) ──
    #[test]
    fn fval_rules() {
        let p = |v: &str| parse(&[&format!("v=FORSALE1;fval={v}")]).unwrap().prices;
        assert_eq!(p("EUR999")[0].amount, "999");
        assert_eq!(p("BTC0.00010")[0].amount, "0.00010");
        assert_eq!(p(" USD5 ")[0].currency, "USD");
        for bad in [
            "eur999", "EUR", "999", "EUR9.", "EUR.5", "EUR 999", "EUR9,99", "E",
        ] {
            assert!(p(bad).is_empty(), "{bad} should be rejected");
        }
    }

    // ── furi (§2.2.3) ──
    #[test]
    fn furi_trust() {
        let u = |v: &str| parse(&[&format!("v=FORSALE1;furi={v}")]).unwrap().uris;
        assert!(u("https://a.example/")[0].trusted);
        assert!(u("http://a.example")[0].trusted);
        assert!(u("mailto:sales@example.com")[0].trusted);
        assert!(u("tel:+31123456789")[0].trusted);
        let ftp = u("ftp://a.example/");
        assert!(!ftp[0].trusted);
        assert_eq!(ftp[0].scheme, "ftp");
        assert!(!u("javascript:alert(1)")[0].trusted);
        assert_eq!(u("HTTPS://A.example")[0].scheme, "https");
        for bad in [
            "https:///nohost",
            "not a uri",
            "https://a b",
            "noscheme",
            "1http://x",
            ":x",
        ] {
            let uris = u(bad);
            assert!(uris.iter().all(|x| !x.trusted), "{bad} must not be trusted");
        }
        assert!(u("not a uri").is_empty());
        assert!(u("noscheme").is_empty());
    }

    // ── sanitize (§3.6, §4) ──
    #[test]
    fn sanitize_strips_terminal_escapes() {
        assert_eq!(sanitize("a\u{1b}[31mred\u{1b}[0m"), "a[31mred[0m");
        assert_eq!(sanitize("x\u{9b}31my"), "x31my");
        assert_eq!(sanitize("bell\u{7}"), "bell");
        assert_eq!(sanitize("a\tb\nc\rd"), "a b c d");
    }

    #[test]
    fn sanitize_strips_bidi_controls() {
        assert_eq!(sanitize("abc\u{202E}fed"), "abcfed");
        assert_eq!(sanitize("\u{2066}x\u{2069}\u{200F}\u{061C}"), "x");
    }

    #[test]
    fn sanitize_strips_invisible_format_characters() {
        // Unicode tag characters ("ASCII smuggling") hide text from humans but not LLMs.
        let smuggled: String = "Great"
            .chars()
            .chain(
                "ignore previous"
                    .chars()
                    .map(|c| char::from_u32(0xE0000 + c as u32).unwrap()),
            )
            .collect();
        assert_eq!(sanitize(&smuggled), "Great");
        assert_eq!(
            sanitize("a\u{200B}b\u{2060}c\u{FEFF}d\u{00AD}e\u{180E}f"),
            "abcdef"
        );
        assert_eq!(sanitize("line\u{2028}para\u{2029}end"), "line para end");
        // Joiners stay: they shape emoji sequences and some scripts.
        assert_eq!(sanitize("👩\u{200D}💻"), "👩\u{200D}💻");
        assert_eq!(sanitize("می\u{200C}خواهم"), "می\u{200C}خواهم");
    }

    #[test]
    fn text_that_sanitizes_to_empty_is_dropped() {
        let info = parse(&["v=FORSALE1;ftxt=\u{1b}\u{7}"]).unwrap();
        assert!(info.texts.is_empty());
    }

    // ── record_name (§2.6) ──
    #[test]
    fn record_name_rules() {
        assert_eq!(
            record_name("example.com").as_deref(),
            Some("_for-sale.example.com.")
        );
        assert_eq!(
            record_name("Example.COM.").as_deref(),
            Some("_for-sale.example.com.")
        );
        assert_eq!(record_name("51.198.in-addr.arpa"), None);
        assert_eq!(record_name("arpa"), None);
        assert_eq!(record_name(""), None);
        assert_eq!(record_name("."), None);
    }

    #[test]
    fn json_shape_omits_empty_lists() {
        let info = parse(&["v=FORSALE1;fval=EUR5"]).unwrap();
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"prices":[{"currency":"EUR","amount":"5"}]})
        );
        let empty = serde_json::to_value(parse(&["v=FORSALE1;"]).unwrap()).unwrap();
        assert_eq!(empty, serde_json::json!({}));
    }

    // ── ForSaleLookup (no network) ──
    #[cfg(feature = "forsale")]
    fn taken(domain: &str) -> crate::types::DomainResult {
        crate::types::DomainResult {
            domain: domain.to_string(),
            available: Some(false),
            info: None,
            check_duration: None,
            method_used: crate::types::CheckMethod::Rdap,
            error_message: None,
            for_sale: None,
        }
    }

    #[test]
    fn lookup_inactive_when_not_requested() {
        let lookup = ForSaleLookup::new(&crate::types::CheckConfig::default());
        assert!(!lookup.is_active());
    }

    #[cfg(not(feature = "forsale"))]
    #[test]
    fn lookup_inactive_without_feature() {
        let config = crate::types::CheckConfig::default().with_for_sale(true);
        assert!(!ForSaleLookup::new(&config).is_active());
    }

    #[cfg(feature = "forsale")]
    #[tokio::test]
    async fn annotate_skips_non_taken_results() {
        let config = crate::types::CheckConfig::default().with_for_sale(true);
        let lookup = ForSaleLookup::new(&config);
        for available in [Some(true), None] {
            let mut r = taken("example.nl");
            r.available = available;
            lookup.annotate(&mut r).await;
            assert!(r.for_sale.is_none());
        }
    }

    #[cfg(feature = "forsale")]
    #[tokio::test]
    async fn annotate_times_out_without_touching_verdict() {
        let config = crate::types::CheckConfig::default().with_for_sale(true);
        let mut lookup = ForSaleLookup::new(&config);
        assert!(lookup.is_active(), "system resolver should be available");
        lookup.timeout = std::time::Duration::ZERO;
        let mut r = taken("example.nl");
        r.error_message = Some("kept".into());
        lookup.annotate(&mut r).await;
        assert!(r.for_sale.is_none());
        assert_eq!(r.available, Some(false));
        assert_eq!(r.method_used, crate::types::CheckMethod::Rdap);
        assert_eq!(r.error_message.as_deref(), Some("kept"));
    }

    /// Exercises DNS + parsing directly, independent of RDAP (which SIDN rate-limits).
    #[cfg(feature = "forsale")]
    #[tokio::test]
    async fn dns_lookup_example_nl_without_rdap() {
        let config = crate::types::CheckConfig::default().with_for_sale(true);
        let for_sale = ForSaleLookup::new(&config);
        let resolver = for_sale.resolver.as_ref().expect("system resolver");
        let info = lookup(resolver, "example.nl", LOOKUP_TIMEOUT)
            .await
            .expect("_for-sale.example.nl is published by SIDN");
        assert!(info.prices.iter().any(|p| p.to_string() == "EUR 100000000"));
        assert!(info.first_trusted_uri().is_some());
        assert!(!info.codes.is_empty());
    }

    /// A nameserver that never answers (TEST-NET-1): the lookup must give up
    /// at our timeout and leave the verdict untouched.
    #[cfg(feature = "forsale")]
    #[tokio::test]
    async fn hanging_resolver_is_capped_and_verdict_unchanged() {
        let timeout = std::time::Duration::from_millis(500);
        let for_sale = ForSaleLookup::with_nameserver("192.0.2.1".parse().unwrap(), timeout);
        assert!(for_sale.is_active());
        let mut r = taken("example.nl");
        r.error_message = Some("kept".into());
        let start = std::time::Instant::now();
        for_sale.annotate(&mut r).await;
        assert!(
            start.elapsed() < std::time::Duration::from_secs(2),
            "lookup took {:?}",
            start.elapsed()
        );
        assert!(r.for_sale.is_none());
        assert_eq!(r.available, Some(false));
        assert_eq!(r.method_used, crate::types::CheckMethod::Rdap);
        assert_eq!(r.error_message.as_deref(), Some("kept"));
    }
}
