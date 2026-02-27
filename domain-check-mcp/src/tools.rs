use domain_check_lib::{
    generate_names, get_available_presets, get_preset_tlds, CheckConfig, DomainChecker,
    GenerateConfig,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo, ToolsCapability},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Safety limits ────────────────────────────────────────────────────────

const MAX_BATCH_DOMAINS: usize = 500;
const MAX_GENERATED_NAMES: usize = 100_000;

// ── Parameter structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckDomainParams {
    #[schemars(description = "Fully qualified domain name to check (e.g. \"example.com\")")]
    pub domain: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckDomainsParams {
    #[schemars(description = "List of fully qualified domain names to check")]
    pub domains: Vec<String>,

    #[schemars(description = "Max concurrent checks (1-100, default 20)")]
    pub concurrency: Option<usize>,

    #[schemars(description = "Timeout per domain in seconds (default 5)")]
    pub timeout_secs: Option<u64>,
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
}

#[derive(Serialize)]
struct BatchCheckResponse {
    total: usize,
    available: usize,
    taken: usize,
    unknown: usize,
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

    #[tool(description = "Check if a single domain name is available for registration")]
    async fn check_domain(
        &self,
        Parameters(params): Parameters<CheckDomainParams>,
    ) -> Result<String, String> {
        match self.checker.check_domain(&params.domain).await {
            Ok(r) => Ok(to_json(&DomainCheckResponse {
                domain: r.domain,
                available: r.available,
                method: r.method_used.to_string(),
                error: r.error_message,
            })),
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(
        description = "Check availability of multiple domain names concurrently. Max 500 domains per call."
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

        let checker = if params.concurrency.is_some() || params.timeout_secs.is_some() {
            DomainChecker::with_config(
                CheckConfig::default()
                    .with_concurrency(params.concurrency.unwrap_or(20))
                    .with_timeout(Duration::from_secs(params.timeout_secs.unwrap_or(5))),
            )
        } else {
            self.checker.clone()
        };

        match checker.check_domains(&params.domains).await {
            Ok(results) => Ok(to_json(&to_batch_response(results))),
            Err(e) => Err(e.to_string()),
        }
    }

    #[tool(
        description = "Check a base name across all TLDs in a preset (e.g. \"startup\", \"tech\", \"popular\"). Use list_presets to see available presets."
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

        let checker = if let Some(c) = params.concurrency {
            DomainChecker::with_config(CheckConfig::default().with_concurrency(c))
        } else {
            self.checker.clone()
        };

        match checker.check_domains(&domains).await {
            Ok(results) => Ok(to_json(&to_batch_response(results))),
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
        description = "Get detailed registration information for a domain (registrar, dates, nameservers, status)"
    )]
    async fn domain_info(
        &self,
        Parameters(params): Parameters<DomainInfoParams>,
    ) -> Result<String, String> {
        let config = CheckConfig::default().with_detailed_info(true);
        let checker = DomainChecker::with_config(config);

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
                    error: r.error_message,
                }))
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DomainCheckServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "domain-check-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            instructions: Some(
                "Domain availability checking tools. Check single or batch domains, \
                 generate name candidates from patterns, and get detailed registration info."
                    .into(),
            ),
            ..Default::default()
        }
    }
}

fn to_batch_response(results: Vec<domain_check_lib::DomainResult>) -> BatchCheckResponse {
    let responses: Vec<DomainCheckResponse> = results
        .into_iter()
        .map(|r| DomainCheckResponse {
            domain: r.domain,
            available: r.available,
            method: r.method_used.to_string(),
            error: r.error_message,
        })
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

    BatchCheckResponse {
        total: responses.len(),
        available,
        taken,
        unknown,
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
                },
                DomainCheckResponse {
                    domain: "taken.com".into(),
                    available: Some(false),
                    method: "RDAP".into(),
                    error: None,
                },
                DomainCheckResponse {
                    domain: "unknown.xyz".into(),
                    available: None,
                    method: "Unknown".into(),
                    error: Some("failed".into()),
                },
            ],
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
        let batch = to_batch_response(vec![]);
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
            },
            DomainResult {
                domain: "taken.com".into(),
                available: Some(false),
                info: None,
                check_duration: None,
                method_used: CheckMethod::Whois,
                error_message: None,
            },
            DomainResult {
                domain: "err.xyz".into(),
                available: None,
                info: None,
                check_duration: None,
                method_used: CheckMethod::Unknown,
                error_message: Some("timeout".into()),
            },
        ];
        let batch = to_batch_response(results);
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
            },
            DomainResult {
                domain: "b.com".into(),
                available: Some(true),
                info: None,
                check_duration: None,
                method_used: CheckMethod::Rdap,
                error_message: None,
            },
        ];
        let batch = to_batch_response(results);
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
            model::{CallToolRequestParams, ClientInfo},
            service::RunningService,
            ClientHandler, RoleClient, ServiceExt,
        };

        type Client = RunningService<RoleClient, TestClient>;

        #[derive(Debug, Clone, Default)]
        struct TestClient;

        impl ClientHandler for TestClient {
            fn get_info(&self) -> ClientInfo {
                ClientInfo::default()
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
                .and_then(|c| c.raw.as_text())
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
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "list_presets".into(),
                    arguments: Some(serde_json::Map::new()),
                    task: None,
                })
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
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "generate_names".into(),
                    arguments: Some(
                        serde_json::json!({
                            "patterns": ["app\\d\\d"]
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                    task: None,
                })
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
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "generate_names".into(),
                    arguments: Some(
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
                    task: None,
                })
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
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "generate_names".into(),
                    arguments: Some(
                        serde_json::json!({
                            "patterns": ["bad\\x"]
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                    task: None,
                })
                .await
                .expect("call_tool failed");

            // Should be an error result
            assert_eq!(result.is_error, Some(true));
            let text = text_from_result(&result);
            assert!(text.contains("unknown escape"));

            client.cancel().await.expect("cancel failed");
        }

        #[tokio::test]
        async fn test_duplex_check_domains_empty_list() {
            let client = setup_client().await;

            let result = client
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "check_domains".into(),
                    arguments: Some(
                        serde_json::json!({
                            "domains": []
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                    task: None,
                })
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
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "check_with_preset".into(),
                    arguments: Some(
                        serde_json::json!({
                            "name": "test",
                            "preset": "does_not_exist"
                        })
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                    task: None,
                })
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
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: "nonexistent_tool".into(),
                    arguments: Some(serde_json::Map::new()),
                    task: None,
                })
                .await;

            // Should return an error (either protocol error or tool error)
            assert!(
                result.is_err() || result.as_ref().unwrap().is_error == Some(true),
                "Calling nonexistent tool should fail"
            );

            client.cancel().await.expect("cancel failed");
        }
    }
}
