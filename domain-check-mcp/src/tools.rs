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
