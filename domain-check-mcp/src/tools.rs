use domain_check_lib::{
    generate_names, get_available_presets, get_preset_tlds, load_env_config, CheckConfig,
    DomainChecker, ForSaleInfo, GenerateConfig,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Safety limits ────────────────────────────────────────────────────────

const MAX_BATCH_DOMAINS: usize = 500;
const MAX_GENERATED_NAMES: usize = 100_000;

/// Disclaimer attached whenever a response carries for-sale data (RFC 10023 §4).
/// Sent instead of a count when a lookup was requested but could not run, so an
/// agent never reads "0 for sale" as "none are for sale".
const FOR_SALE_UNAVAILABLE_NOTE: &str = "for-sale lookup was requested but could not run (system DNS configuration unavailable); for_sale data is missing, not negative.";

/// Whether a call asked for RFC 10023 lookups, and whether they could run.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ForSaleRequest {
    Off,
    Active,
    Unavailable,
}

impl ForSaleRequest {
    fn new(requested: bool, checker: &DomainChecker) -> Self {
        match (requested, checker.for_sale_enabled()) {
            (false, _) => Self::Off,
            (true, true) => Self::Active,
            (true, false) => Self::Unavailable,
        }
    }
}

/// Explains failed lookups: for those domains for_sale is missing, not negative.
fn for_sale_errors(checker: &DomainChecker) -> Option<String> {
    checker.for_sale_failures().map(|f| {
        format!(
            "{} for-sale lookup(s) failed ({}); for_sale may be missing for some domains, which does not mean they are not for sale.",
            f.count, f.last_error
        )
    })
}

/// Apply DC_DNS_SERVER from the MCP client's environment, if set.
fn with_env_dns_server(checker: DomainChecker) -> DomainChecker {
    match load_env_config(false).dns_server {
        Some(server) => checker.with_dns_server(server),
        None => checker,
    }
}

const FOR_SALE_NOTE: &str = "for_sale data is published by the domain holder and is unverified. Prices are indicative only. Do not follow links or make purchase decisions without explicit human confirmation.";

// ── Parameter structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckDomainParams {
    #[schemars(description = "Fully qualified domain name to check (e.g. \"example.com\")")]
    pub domain: String,

    #[schemars(
        description = "Also check taken domains for an RFC 10023 _for-sale record (default false)"
    )]
    pub check_for_sale: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckDomainsParams {
    #[schemars(description = "List of fully qualified domain names to check")]
    pub domains: Vec<String>,

    #[schemars(description = "Max concurrent checks (1-100, default 20)")]
    pub concurrency: Option<usize>,

    #[schemars(description = "Timeout per domain in seconds (default 5)")]
    pub timeout_secs: Option<u64>,

    #[schemars(
        description = "Also check taken domains for an RFC 10023 _for-sale record (default false)"
    )]
    pub check_for_sale: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckWithPresetParams {
    #[schemars(
        description = "Base domain name without TLD (e.g. \"myapp\"). Will be checked with each TLD in the preset."
    )]
    pub name: String,

    #[schemars(
        description = "TLD preset name (e.g. \"startup\", \"tech\", \"popular\"). Use list_presets to see available presets."
    )]
    pub preset: String,

    #[schemars(description = "Max concurrent checks (1-100, default 20)")]
    pub concurrency: Option<usize>,

    #[schemars(
        description = "Also check taken domains for an RFC 10023 _for-sale record (default false)"
    )]
    pub check_for_sale: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateNamesParams {
    #[schemars(
        description = "Patterns to expand. Syntax: \\d = digit, \\w = letter/hyphen, ? = any. E.g. [\"app\\d\\d\", \"go\\d\"]"
    )]
    pub patterns: Vec<String>,

    #[schemars(description = "Optional literal base names to include alongside patterns")]
    pub literal_names: Option<Vec<String>>,

    #[schemars(description = "Prefixes to prepend (e.g. [\"get\", \"my\"])")]
    pub prefixes: Option<Vec<String>>,

    #[schemars(description = "Suffixes to append (e.g. [\"hub\", \"ly\"])")]
    pub suffixes: Option<Vec<String>>,

    #[schemars(description = "Include bare name without affixes (default true)")]
    pub include_bare: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DomainInfoParams {
    #[schemars(description = "Fully qualified domain name to get registration info for")]
    pub domain: String,
}

// ── Response structs ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct DomainCheckResponse {
    domain: String,
    available: Option<bool>,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale: Option<ForSaleInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_note: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_errors: Option<String>,
}

#[derive(Serialize)]
struct BatchCheckResponse {
    total: usize,
    available: usize,
    taken: usize,
    unknown: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_note: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_errors: Option<String>,
    results: Vec<DomainCheckResponse>,
}

#[derive(Serialize)]
struct GenerateNamesResponse {
    count: usize,
    estimated_before_filter: usize,
    names: Vec<String>,
}

#[derive(Serialize)]
struct PresetInfo {
    name: String,
    tlds: Vec<String>,
}

#[derive(Serialize)]
struct ListPresetsResponse {
    presets: Vec<PresetInfo>,
}

#[derive(Serialize)]
struct DomainInfoResponse {
    domain: String,
    available: Option<bool>,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    registrar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiration_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nameservers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale: Option<ForSaleInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_note: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    for_sale_errors: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

// ── Server ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DomainCheckServer {
    checker: DomainChecker,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl DomainCheckServer {
    pub fn new() -> Self {
        Self {
            checker: DomainChecker::new(),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Check if a single domain name is available for registration. Set check_for_sale to also detect taken domains advertised for sale (RFC 10023)."
    )]
    async fn check_domain(
        &self,
        Parameters(params): Parameters<CheckDomainParams>,
    ) -> Result<String, String> {
        let requested = params.check_for_sale.unwrap_or(false);
        let checker = self.checker_for(None, None, requested);
        let request = ForSaleRequest::new(requested, &checker);
        match checker.check_domain(&params.domain).await {
            Ok(r) => {
                let mut response = to_check_response(r, request, true);
                response.for_sale_errors = for_sale_errors(&checker);
                Ok(to_json(&response))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(
        description = "Check availability of multiple domain names concurrently. Max 500 domains per call. Set check_for_sale to detect taken domains advertised for sale (RFC 10023)."
    )]
    async fn check_domains(
        &self,
        Parameters(params): Parameters<CheckDomainsParams>,
    ) -> Result<String, String> {
        if params.domains.is_empty() {
            return Err("domains list cannot be empty".into());
        }
        if params.domains.len() > MAX_BATCH_DOMAINS {
            return Err(format!(
                "Too many domains ({}). Maximum is {MAX_BATCH_DOMAINS}.",
                params.domains.len()
            ));
        }

        let for_sale = params.check_for_sale.unwrap_or(false);
        let checker = self.checker_for(params.concurrency, params.timeout_secs, for_sale);

        let request = ForSaleRequest::new(for_sale, &checker);
        match checker.check_domains(&params.domains).await {
            Ok(results) => {
                let mut response = to_batch_response(results, request);
                response.for_sale_errors = for_sale_errors(&checker);
                Ok(to_json(&response))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(
        description = "Check a base name across all TLDs in a preset (e.g. \"startup\", \"tech\", \"popular\"). Use list_presets to see available presets. Set check_for_sale to detect taken domains advertised for sale (RFC 10023)."
    )]
    async fn check_with_preset(
        &self,
        Parameters(params): Parameters<CheckWithPresetParams>,
    ) -> Result<String, String> {
        let tlds = match get_preset_tlds(&params.preset) {
            Some(tlds) => tlds,
            None => {
                let available = get_available_presets().join(", ");
                return Err(format!(
                    "Unknown preset \"{}\". Available: {available}",
                    params.preset
                ));
            }
        };

        let domains: Vec<String> = tlds
            .iter()
            .map(|tld| format!("{}.{}", params.name, tld))
            .collect();

        let for_sale = params.check_for_sale.unwrap_or(false);
        let checker = self.checker_for(params.concurrency, None, for_sale);
        let request = ForSaleRequest::new(for_sale, &checker);

        match checker.check_domains(&domains).await {
            Ok(results) => {
                let mut response = to_batch_response(results, request);
                response.for_sale_errors = for_sale_errors(&checker);
                Ok(to_json(&response))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(
        description = "Generate domain name candidates from patterns and optional prefixes/suffixes. Pattern syntax: \\d = digit (0-9), \\w = letter (a-z) or hyphen, ? = any of the above."
    )]
    async fn generate_names(
        &self,
        Parameters(params): Parameters<GenerateNamesParams>,
    ) -> Result<String, String> {
        let config = GenerateConfig {
            patterns: params.patterns,
            prefixes: params.prefixes.unwrap_or_default(),
            suffixes: params.suffixes.unwrap_or_default(),
            include_bare: params.include_bare.unwrap_or(true),
        };

        let literals = params.literal_names.unwrap_or_default();

        match generate_names(&config, &literals) {
            Ok(result) => {
                if result.names.len() > MAX_GENERATED_NAMES {
                    return Err(format!(
                        "Pattern would generate {} names, exceeding limit of {MAX_GENERATED_NAMES}. Use more specific patterns.",
                        result.names.len()
                    ));
                }
                Ok(to_json(&GenerateNamesResponse {
                    count: result.names.len(),
                    estimated_before_filter: result.estimated_count,
                    names: result.names,
                }))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(description = "List all available TLD presets and the TLDs they contain")]
    async fn list_presets(&self) -> String {
        let preset_names = get_available_presets();
        let presets: Vec<PresetInfo> = preset_names
            .into_iter()
            .map(|name| PresetInfo {
                tlds: get_preset_tlds(name).unwrap_or_default(),
                name: name.to_string(),
            })
            .collect();

        to_json(&ListPresetsResponse { presets })
    }

    #[tool(
        description = "Get detailed registration information for a domain (registrar, dates, nameservers, status, and RFC 10023 for-sale info if published)"
    )]
    async fn domain_info(
        &self,
        Parameters(params): Parameters<DomainInfoParams>,
    ) -> Result<String, String> {
        let config = CheckConfig::default()
            .with_detailed_info(true)
            .with_for_sale(true);
        let checker = with_env_dns_server(DomainChecker::with_config(config));

        match checker.check_domain(&params.domain).await {
            Ok(r) => {
                let info = r.info.as_ref();
                Ok(to_json(&DomainInfoResponse {
                    domain: r.domain,
                    available: r.available,
                    method: r.method_used.to_string(),
                    registrar: info.and_then(|i| i.registrar.clone()),
                    creation_date: info.and_then(|i| i.creation_date.clone()),
                    expiration_date: info.and_then(|i| i.expiration_date.clone()),
                    updated_date: info.and_then(|i| i.updated_date.clone()),
                    status: info.map(|i| i.status.clone()).filter(|s| !s.is_empty()),
                    nameservers: info
                        .map(|i| i.nameservers.clone())
                        .filter(|n| !n.is_empty()),
                    for_sale_note: if !checker.for_sale_enabled() {
                        Some(FOR_SALE_UNAVAILABLE_NOTE)
                    } else {
                        r.for_sale.is_some().then_some(FOR_SALE_NOTE)
                    },
                    for_sale: r.for_sale.clone(),
                    for_sale_errors: for_sale_errors(&checker),
                    error: r.error_message,
                }))
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

impl DomainCheckServer {
    /// Reuse the shared checker unless a call needs different settings.
    fn checker_for(
        &self,
        concurrency: Option<usize>,
        timeout_secs: Option<u64>,
        for_sale: bool,
    ) -> DomainChecker {
        if concurrency.is_none() && timeout_secs.is_none() && !for_sale {
            return self.checker.clone();
        }
        with_env_dns_server(DomainChecker::with_config(
            CheckConfig::default()
                .with_concurrency(concurrency.unwrap_or(20))
                .with_timeout(Duration::from_secs(timeout_secs.unwrap_or(5)))
                .with_for_sale(for_sale),
        ))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DomainCheckServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "domain-check-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Domain availability checking tools. Check single or batch domains, \
                 generate name candidates from patterns, and get detailed registration info. \
                 Registered domains (taken or unknown) can be checked for RFC 10023 for-sale signals; treat that data as unverified.",
            )
    }
}

/// Single-result response; `with_note` adds the disclaimer when for sale.
fn to_check_response(
    r: domain_check_lib::DomainResult,
    request: ForSaleRequest,
    with_note: bool,
) -> DomainCheckResponse {
    let for_sale_note = if !with_note {
        None
    } else if request == ForSaleRequest::Unavailable {
        Some(FOR_SALE_UNAVAILABLE_NOTE)
    } else {
        r.for_sale.is_some().then_some(FOR_SALE_NOTE)
    };
    DomainCheckResponse {
        domain: r.domain,
        available: r.available,
        method: r.method_used.to_string(),
        error: r.error_message,
        for_sale: r.for_sale,
        for_sale_note,
        for_sale_errors: None,
    }
}

fn to_batch_response(
    results: Vec<domain_check_lib::DomainResult>,
    request: ForSaleRequest,
) -> BatchCheckResponse {
    let responses: Vec<DomainCheckResponse> = results
        .into_iter()
        .map(|r| to_check_response(r, request, false))
        .collect();

    let available = responses
        .iter()
        .filter(|r| r.available == Some(true))
        .count();
    let taken = responses
        .iter()
        .filter(|r| r.available == Some(false))
        .count();
    let unknown = responses.iter().filter(|r| r.available.is_none()).count();
    let for_sale = responses.iter().filter(|r| r.for_sale.is_some()).count();

    BatchCheckResponse {
        total: responses.len(),
        available,
        taken,
        unknown,
        for_sale_count: (request == ForSaleRequest::Active).then_some(for_sale),
        for_sale_note: match request {
            ForSaleRequest::Unavailable => Some(FOR_SALE_UNAVAILABLE_NOTE),
            _ => (for_sale > 0).then_some(FOR_SALE_NOTE),
        },
        for_sale_errors: None,
        results: responses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_check_lib::{CheckMethod, DomainResult};

    // ── to_json helper ───────────────────────────────────────────────────

    #[test]
    fn test_to_json_produces_valid_json() {
        let resp = DomainCheckResponse {
            domain: "example.com".into(),
            available: Some(true),
            method: "RDAP".into(),
            error: None,
            for_sale: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["domain"], "example.com");
        assert_eq!(parsed["available"], true);
        assert_eq!(parsed["method"], "RDAP");
    }

    // ── DomainCheckResponse serialization ────────────────────────────────

    #[test]
    fn test_domain_check_response_skips_none_error() {
        let resp = DomainCheckResponse {
            domain: "test.com".into(),
            available: Some(false),
            method: "WHOIS".into(),
            error: None,
            for_sale: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_domain_check_response_includes_error_when_present() {
        let resp = DomainCheckResponse {
            domain: "test.com".into(),
            available: None,
            method: "Unknown".into(),
            error: Some("network timeout".into()),
            for_sale: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "network timeout");
        assert!(parsed["available"].is_null());
    }

    // ── BatchCheckResponse serialization ─────────────────────────────────

    #[test]
    fn test_batch_response_counts() {
        let resp = BatchCheckResponse {
            total: 3,
            available: 1,
            taken: 1,
            unknown: 1,
            results: vec![
                DomainCheckResponse {
                    domain: "free.com".into(),
                    available: Some(true),
                    method: "RDAP".into(),
                    error: None,
                    for_sale: None,
                    for_sale_note: None,
                    for_sale_errors: None,
                },
                DomainCheckResponse {
                    domain: "taken.com".into(),
                    available: Some(false),
                    method: "RDAP".into(),
                    error: None,
                    for_sale: None,
                    for_sale_note: None,
                    for_sale_errors: None,
                },
                DomainCheckResponse {
                    domain: "unknown.xyz".into(),
                    available: None,
                    method: "Unknown".into(),
                    error: Some("failed".into()),
                    for_sale: None,
                    for_sale_note: None,
                    for_sale_errors: None,
                },
            ],
            for_sale_count: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total"], 3);
        assert_eq!(parsed["available"], 1);
        assert_eq!(parsed["taken"], 1);
        assert_eq!(parsed["unknown"], 1);
        assert_eq!(parsed["results"].as_array().unwrap().len(), 3);
    }

    // ── GenerateNamesResponse serialization ──────────────────────────────

    #[test]
    fn test_generate_names_response() {
        let resp = GenerateNamesResponse {
            count: 2,
            estimated_before_filter: 5,
            names: vec!["app01".into(), "app02".into()],
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["count"], 2);
        assert_eq!(parsed["estimated_before_filter"], 5);
        assert_eq!(parsed["names"].as_array().unwrap().len(), 2);
    }

    // ── ListPresetsResponse serialization ────────────────────────────────

    #[test]
    fn test_list_presets_response() {
        let resp = ListPresetsResponse {
            presets: vec![PresetInfo {
                name: "startup".into(),
                tlds: vec!["com".into(), "io".into()],
            }],
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["presets"][0]["name"], "startup");
        assert_eq!(parsed["presets"][0]["tlds"][0], "com");
    }

    // ── DomainInfoResponse serialization ─────────────────────────────────

    #[test]
    fn test_domain_info_response_minimal() {
        let resp = DomainInfoResponse {
            domain: "available.com".into(),
            available: Some(true),
            method: "RDAP".into(),
            registrar: None,
            creation_date: None,
            expiration_date: None,
            updated_date: None,
            status: None,
            nameservers: None,
            error: None,
            for_sale: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["domain"], "available.com");
        assert_eq!(parsed["available"], true);
        // None fields should be absent
        assert!(parsed.get("registrar").is_none());
        assert!(parsed.get("nameservers").is_none());
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_domain_info_response_full() {
        let resp = DomainInfoResponse {
            domain: "google.com".into(),
            available: Some(false),
            method: "RDAP".into(),
            registrar: Some("MarkMonitor Inc.".into()),
            creation_date: Some("1997-09-15".into()),
            expiration_date: Some("2028-09-14".into()),
            updated_date: Some("2019-09-09".into()),
            status: Some(vec!["clientTransferProhibited".into()]),
            nameservers: Some(vec!["ns1.google.com".into()]),
            error: None,
            for_sale: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["registrar"], "MarkMonitor Inc.");
        assert_eq!(parsed["nameservers"][0], "ns1.google.com");
        assert_eq!(parsed["status"][0], "clientTransferProhibited");
    }

    #[test]
    fn test_domain_info_response_skips_empty_vectors() {
        let resp = DomainInfoResponse {
            domain: "test.com".into(),
            available: Some(false),
            method: "RDAP".into(),
            registrar: Some("Test".into()),
            creation_date: None,
            expiration_date: None,
            updated_date: None,
            status: Some(vec![]),      // empty vec should be skipped
            nameservers: Some(vec![]), // empty vec should be skipped
            error: None,
            for_sale: None,
            for_sale_note: None,
            for_sale_errors: None,
        };
        let json = to_json(&resp);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // empty vecs are serialized as [] since skip_serializing_if is on Option, not on empty
        // but our code uses .filter(|s| !s.is_empty()) so they become None before reaching here
        // However this test constructs directly — so the filter doesn't apply.
        // This tests the serde behavior: Some(vec![]) IS serialized.
        assert!(parsed.get("status").is_some());
    }

    // ── to_batch_response helper ─────────────────────────────────────────

    #[test]
    fn test_to_batch_response_empty() {
        let batch = to_batch_response(vec![], ForSaleRequest::Off);
        assert_eq!(batch.total, 0);
        assert_eq!(batch.available, 0);
        assert_eq!(batch.taken, 0);
        assert_eq!(batch.unknown, 0);
        assert!(batch.results.is_empty());
    }

    #[test]
    fn test_to_batch_response_mixed_results() {
        let results = vec![
            DomainResult {
                domain: "free.com".into(),
                available: Some(true),
                info: None,
                check_duration: None,
                method_used: CheckMethod::Rdap,
                error_message: None,
                for_sale: None,
            },
            DomainResult {
                domain: "taken.com".into(),
                available: Some(false),
                info: None,
                check_duration: None,
                method_used: CheckMethod::Whois,
                error_message: None,
                for_sale: None,
            },
            DomainResult {
                domain: "err.xyz".into(),
                available: None,
                info: None,
                check_duration: None,
                method_used: CheckMethod::Unknown,
                error_message: Some("timeout".into()),
                for_sale: None,
            },
        ];
        let batch = to_batch_response(results, ForSaleRequest::Off);
        assert_eq!(batch.total, 3);
        assert_eq!(batch.available, 1);
        assert_eq!(batch.taken, 1);
        assert_eq!(batch.unknown, 1);
        assert_eq!(batch.results[0].domain, "free.com");
        assert_eq!(batch.results[1].method, "WHOIS");
        assert_eq!(batch.results[2].error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_to_batch_response_all_available() {
        let results = vec![
            DomainResult {
                domain: "a.com".into(),
                available: Some(true),
                info: None,
                check_duration: None,
                method_used: CheckMethod::Rdap,
                error_message: None,
                for_sale: None,
            },
            DomainResult {
                domain: "b.com".into(),
                available: Some(true),
                info: None,
                check_duration: None,
                method_used: CheckMethod::Rdap,
                error_message: None,
                for_sale: None,
            },
        ];
        let batch = to_batch_response(results, ForSaleRequest::Off);
        assert_eq!(batch.available, 2);
        assert_eq!(batch.taken, 0);
        assert_eq!(batch.unknown, 0);
    }

    // ── Server construction & info ───────────────────────────────────────

    #[test]
    fn test_server_new() {
        let server = DomainCheckServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "domain-check-mcp");
        assert!(!info.server_info.version.is_empty());
    }

    #[test]
    fn test_server_info_has_tools_capability() {
        let server = DomainCheckServer::new();
        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "Server must advertise tools capability"
        );
    }

    #[test]
    fn test_server_info_has_instructions() {
        let server = DomainCheckServer::new();
        let info = server.get_info();
        assert!(info.instructions.is_some());
        assert!(info.instructions.unwrap().contains("domain"));
    }

    #[test]
    fn test_server_info_version_matches_crate() {
        let server = DomainCheckServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    }

    // ── Tool registration ────────────────────────────────────────────────

    #[test]
    fn test_tool_router_has_six_tools() {
        let server = DomainCheckServer::new();
        let tools = server.tool_router.list_all();
        assert_eq!(tools.len(), 6, "Expected 6 tools, got {}", tools.len());
    }

    fn for_sale_result(domain: &str) -> DomainResult {
        DomainResult {
            domain: domain.into(),
            available: Some(false),
            info: None,
            check_duration: None,
            method_used: CheckMethod::Rdap,
            error_message: None,
            for_sale: domain_check_lib::parse_txt_records(&[
                vec![b"v=FORSALE1;fval=EUR5".to_vec()],
            ]),
        }
    }

    #[test]
    fn test_check_response_includes_for_sale_and_note() {
        let json = to_json(&to_check_response(
            for_sale_result("a.com"),
            ForSaleRequest::Active,
            true,
        ));
        assert!(json.contains("\"for_sale\""));
        assert!(json.contains(FOR_SALE_NOTE));
    }

    #[test]
    fn test_check_response_omits_for_sale_when_absent() {
        let mut r = for_sale_result("a.com");
        r.for_sale = None;
        let json = to_json(&to_check_response(r, ForSaleRequest::Active, true));
        assert!(!json.contains("for_sale"));
    }

    #[test]
    fn test_batch_for_sale_count_and_single_note() {
        let mut plain = for_sale_result("b.com");
        plain.for_sale = None;
        let resp = to_batch_response(
            vec![for_sale_result("a.com"), plain],
            ForSaleRequest::Active,
        );
        assert_eq!(resp.for_sale_count, Some(1));
        let json = to_json(&resp);
        assert_eq!(
            json.matches(FOR_SALE_NOTE).count(),
            1,
            "note appears once, at top level"
        );
    }

    #[test]
    fn test_batch_unavailable_lookup_reports_unavailable_not_zero() {
        let mut plain = for_sale_result("b.com");
        plain.for_sale = None;
        let resp = to_batch_response(vec![plain], ForSaleRequest::Unavailable);
        assert!(resp.for_sale_count.is_none(), "must not claim 0 for sale");
        assert_eq!(resp.for_sale_note, Some(FOR_SALE_UNAVAILABLE_NOTE));
    }

    #[test]
    fn test_check_response_unavailable_lookup_says_so() {
        let mut r = for_sale_result("a.com");
        r.for_sale = None;
        let json = to_json(&to_check_response(r, ForSaleRequest::Unavailable, true));
        assert!(json.contains(FOR_SALE_UNAVAILABLE_NOTE), "{json}");
    }

    #[test]
    fn test_batch_without_lookup_has_no_count() {
        let resp = to_batch_response(vec![], ForSaleRequest::Off);
        assert!(resp.for_sale_count.is_none());
        assert!(!to_json(&resp).contains("for_sale"));
    }

    #[test]
    fn test_check_for_sale_param_in_schema() {
        let server = DomainCheckServer::new();
        let tools = server.tool_router.list_all();
        for name in ["check_domain", "check_domains", "check_with_preset"] {
            let tool = tools.iter().find(|t| t.name == name).unwrap();
            let schema = serde_json::to_string(&tool.input_schema).unwrap();
            assert!(
                schema.contains("check_for_sale"),
                "{name} missing check_for_sale"
            );
        }
    }

    #[test]
    fn test_tool_router_tool_names() {
        let server = DomainCheckServer::new();
        let tools = server.tool_router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"check_domain"));
        assert!(names.contains(&"check_domains"));
        assert!(names.contains(&"check_with_preset"));
        assert!(names.contains(&"generate_names"));
        assert!(names.contains(&"list_presets"));
        assert!(names.contains(&"domain_info"));
    }

    #[test]
    fn test_tool_descriptions_not_empty() {
        let server = DomainCheckServer::new();
        let tools = server.tool_router.list_all();
        for tool in &tools {
            assert!(
                tool.description.is_some(),
                "Tool {} missing description",
                tool.name
            );
            assert!(
                !tool.description.as_ref().unwrap().is_empty(),
                "Tool {} has empty description",
                tool.name
            );
        }
    }

    #[test]
    fn test_tool_schemas_have_type_object() {
        let server = DomainCheckServer::new();
        let tools = server.tool_router.list_all();
        for tool in &tools {
            assert_eq!(
                tool.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "Tool {} input schema must have type: object",
                tool.name
            );
        }
    }

    // ── list_presets tool (no network) ────────────────────────────────────

    #[tokio::test]
    async fn test_list_presets_tool() {
        let server = DomainCheckServer::new();
        let result = server.list_presets().await;
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let presets = parsed["presets"].as_array().unwrap();
        assert!(presets.len() >= 10, "Expected at least 10 presets");

        // Verify each preset has name and non-empty tlds
        for preset in presets {
            assert!(preset["name"].is_string());
            assert!(!preset["tlds"].as_array().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn test_list_presets_contains_known_presets() {
        let server = DomainCheckServer::new();
        let result = server.list_presets().await;
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let names: Vec<&str> = parsed["presets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"startup"));
        assert!(names.contains(&"tech"));
        assert!(names.contains(&"popular"));
        assert!(names.contains(&"classic"));
    }

    // ── generate_names tool (no network) ─────────────────────────────────

    #[tokio::test]
    async fn test_generate_names_tool_simple_pattern() {
        let server = DomainCheckServer::new();
        let result = server
            .generate_names(Parameters(GenerateNamesParams {
                patterns: vec!["app\\d".into()],
                literal_names: None,
                prefixes: None,
                suffixes: None,
                include_bare: None,
            }))
            .await;
        assert!(result.is_ok());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed["count"], 10);
        let names = parsed["names"].as_array().unwrap();
        assert!(names.iter().any(|n| n == "app0"));
        assert!(names.iter().any(|n| n == "app9"));
    }

    #[tokio::test]
    async fn test_generate_names_tool_with_affixes() {
        let server = DomainCheckServer::new();
        let result = server
            .generate_names(Parameters(GenerateNamesParams {
                patterns: vec![],
                literal_names: Some(vec!["cloud".into()]),
                prefixes: Some(vec!["get".into()]),
                suffixes: Some(vec!["ly".into()]),
                include_bare: Some(true),
            }))
            .await;
        assert!(result.is_ok());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let names: Vec<&str> = parsed["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap())
            .collect();
        assert!(names.contains(&"getcloudly"));
        assert!(names.contains(&"getcloud"));
        assert!(names.contains(&"cloudly"));
        assert!(names.contains(&"cloud"));
    }

    #[tokio::test]
    async fn test_generate_names_tool_invalid_pattern() {
        let server = DomainCheckServer::new();
        let result = server
            .generate_names(Parameters(GenerateNamesParams {
                patterns: vec!["test\\x".into()],
                literal_names: None,
                prefixes: None,
                suffixes: None,
                include_bare: None,
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown escape"));
    }

    // ── check_with_preset validation (no network) ────────────────────────

    #[tokio::test]
    async fn test_check_with_preset_unknown_preset() {
        let server = DomainCheckServer::new();
        let result = server
            .check_with_preset(Parameters(CheckWithPresetParams {
                name: "test".into(),
                preset: "nonexistent".into(),
                concurrency: None,
                check_for_sale: None,
            }))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown preset"));
        assert!(err.contains("startup")); // should list available presets
    }

    // ── check_domains validation (no network) ────────────────────────────

    #[tokio::test]
    async fn test_check_domains_empty_list() {
        let server = DomainCheckServer::new();
        let result = server
            .check_domains(Parameters(CheckDomainsParams {
                domains: vec![],
                concurrency: None,
                timeout_secs: None,
                check_for_sale: None,
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_check_domains_exceeds_limit() {
        let server = DomainCheckServer::new();
        let domains: Vec<String> = (0..501).map(|i| format!("domain{i}.com")).collect();
        let result = server
            .check_domains(Parameters(CheckDomainsParams {
                domains,
                concurrency: None,
                timeout_secs: None,
                check_for_sale: None,
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("501"));
    }

    #[tokio::test]
    async fn test_check_domains_at_limit_accepted() {
        // 500 domains should NOT be rejected by validation
        // (it will fail on network, but validation passes)
        let server = DomainCheckServer::new();
        let domains: Vec<String> = (0..500).map(|i| format!("domain{i}.com")).collect();
        let result = server
            .check_domains(Parameters(CheckDomainsParams {
                domains,
                concurrency: None,
                timeout_secs: None,
                check_for_sale: None,
            }))
            .await;
        // Should not get the "Too many domains" error
        if let Err(e) = &result {
            assert!(!e.contains("Too many domains"));
        }
    }

    // ── Safety constants ─────────────────────────────────────────────────

    #[test]
    fn test_max_batch_domains_is_500() {
        assert_eq!(MAX_BATCH_DOMAINS, 500);
    }

    #[test]
    fn test_max_generated_names_is_100k() {
        assert_eq!(MAX_GENERATED_NAMES, 100_000);
    }

    // ── Integration tests: duplex client ↔ server ────────────────────────

    mod integration {
        use super::*;
        use rmcp::{
            model::{CallToolRequestParams, ClientConfig},
            service::RunningService,
            ClientHandler, RoleClient, ServiceExt,
        };

        type Client = RunningService<RoleClient, TestClient>;

        #[derive(Debug, Clone, Default)]
        struct TestClient;

        impl ClientHandler for TestClient {
            fn get_info(&self) -> ClientConfig {
                ClientConfig::default()
            }
        }

        async fn setup_client() -> Client {
            let (server_transport, client_transport) = tokio::io::duplex(65536);

            let server = DomainCheckServer::new();
            tokio::spawn(async move {
                let svc = server
                    .serve(server_transport)
                    .await
                    .expect("server start failed");
                let _ = svc.waiting().await;
            });

            TestClient
                .serve(client_transport)
                .await
                .expect("client start failed")
        }

        fn text_from_result(result: &rmcp::model::CallToolResult) -> &str {
            result
                .content
                .first()
                .and_then(|c| c.as_text())
                .map(|t| t.text.as_str())
                .expect("expected text content in result")
        }

        #[tokio::test]
        async fn test_duplex_initialize_and_list_tools() {
            let client = setup_client().await;

            let tools = client.list_tools(None).await.expect("list_tools failed");
            assert_eq!(tools.tools.len(), 6, "Expected 6 tools");

            let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
            assert!(names.contains(&"check_domain"));
            assert!(names.contains(&"check_domains"));
            assert!(names.contains(&"check_with_preset"));
            assert!(names.contains(&"generate_names"));
            assert!(names.contains(&"list_presets"));
            assert!(names.contains(&"domain_info"));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_list_presets() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("list_presets")
                        .with_arguments(serde_json::Map::new()),
                )
                .await
                .expect("call_tool failed");

            let text = text_from_result(&result);
            let parsed: serde_json::Value =
                serde_json::from_str(text).expect("response is not valid JSON");

            let presets = parsed["presets"]
                .as_array()
                .expect("presets should be array");
            assert!(presets.len() >= 10);

            // Verify startup preset exists and has expected TLDs
            let startup = presets.iter().find(|p| p["name"] == "startup");
            assert!(startup.is_some(), "startup preset should exist");
            let startup_tlds = startup.unwrap()["tlds"].as_array().unwrap();
            assert!(startup_tlds.iter().any(|t| t == "com"));
            assert!(startup_tlds.iter().any(|t| t == "io"));

            // Result should not be an error
            assert_ne!(result.is_error, Some(true));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_generate_names() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("generate_names").with_arguments(
                        serde_json::json!({
                            "patterns": ["app\\d\\d"]
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                )
                .await
                .expect("call_tool failed");

            let text = text_from_result(&result);
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();

            assert_eq!(parsed["count"], 100);
            let names = parsed["names"].as_array().unwrap();
            assert_eq!(names.len(), 100);
            assert!(names.iter().any(|n| n == "app00"));
            assert!(names.iter().any(|n| n == "app99"));
            assert!(names.iter().any(|n| n == "app42"));

            assert_ne!(result.is_error, Some(true));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_generate_names_with_affixes() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("generate_names").with_arguments(
                        serde_json::json!({
                            "patterns": [],
                            "literal_names": ["cloud"],
                            "prefixes": ["get"],
                            "suffixes": ["ly"],
                            "include_bare": true
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                )
                .await
                .expect("call_tool failed");

            let text = text_from_result(&result);
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();

            let names: Vec<&str> = parsed["names"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_str().unwrap())
                .collect();
            assert!(names.contains(&"getcloudly"));
            assert!(names.contains(&"getcloud"));
            assert!(names.contains(&"cloudly"));
            assert!(names.contains(&"cloud"));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_generate_names_invalid_pattern() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("generate_names").with_arguments(
                        serde_json::json!({
                            "patterns": ["bad\\x"]
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                )
                .await
                .expect("call_tool failed");

            // Should be an error result
            assert_eq!(result.is_error, Some(true));
            let text = text_from_result(&result);
            assert!(text.contains("unknown escape"));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_check_domain_for_sale() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("check_domain").with_arguments(
                        serde_json::json!({
                            "domain": "example.nl",
                            "check_for_sale": true
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                )
                .await
                .expect("call_tool failed");

            let text = text_from_result(&result);
            // SIDN rate-limits bursts; for-sale only runs on TAKEN results.
            // example.nl is registered, so anything but "taken" means the
            // RDAP answer was lost (a 429, or a misread WHOIS fallback).
            if !text.contains("\"available\": false") {
                eprintln!("skipped: .nl registry did not confirm example.nl as taken");
            } else {
                assert!(text.contains("\"for_sale\""), "{text}");
                assert!(text.contains("indicative only"), "{text}");
            }

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_check_domains_empty_list() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("check_domains").with_arguments(
                        serde_json::json!({
                            "domains": []
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                )
                .await
                .expect("call_tool failed");

            assert_eq!(result.is_error, Some(true));
            let text = text_from_result(&result);
            assert!(text.contains("cannot be empty"));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_check_with_preset_unknown() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("check_with_preset").with_arguments(
                        serde_json::json!({
                            "name": "test",
                            "preset": "does_not_exist"
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                )
                .await
                .expect("call_tool failed");

            assert_eq!(result.is_error, Some(true));
            let text = text_from_result(&result);
            assert!(text.contains("Unknown preset"));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_call_nonexistent_tool() {
            let client = setup_client().await;

            let result = client
                .call_tool(
                    CallToolRequestParams::new("nonexistent_tool")
                        .with_arguments(serde_json::Map::new()),
                )
                .await;

            // Should return an error (either protocol error or tool error)
            assert!(
                result.is_err() || result.as_ref().unwrap().is_error == Some(true),
                "Calling nonexistent tool should fail"
            );

            client.cancel().await.expect("cancel failed");
        }
    }

    #[tokio::test]
    async fn failed_lookups_are_reported_not_hidden() {
        let fresh = DomainChecker::with_config(CheckConfig::default().with_for_sale(true));
        assert_eq!(for_sale_errors(&fresh), None);

        // 192.0.2.1 (TEST-NET-1) never answers.
        let checker = DomainChecker::with_config(
            CheckConfig::default()
                .with_for_sale(true)
                .with_timeout(Duration::from_millis(500)),
        )
        .with_dns_server("192.0.2.1".parse().unwrap());
        match checker.check_domain("google.com").await {
            Ok(r) if r.available == Some(false) => {
                let note = for_sale_errors(&checker).expect("failure must be reported");
                assert!(note.contains("192.0.2.1"), "{note}");
                assert!(note.contains("does not mean"), "{note}");
            }
            _ => eprintln!("skipped: google.com not confirmed as taken"),
        }
    }
}
