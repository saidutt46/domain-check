# domain-check-lib

**A fast, robust Rust library for checking domain availability using RDAP and WHOIS protocols**

[![Crates.io](https://img.shields.io/crates/v/domain-check-lib.svg)](https://crates.io/crates/domain-check-lib)
[![Documentation](https://docs.rs/domain-check-lib/badge.svg)](https://docs.rs/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check-lib.svg)](https://crates.io/crates/domain-check-lib)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Note: docs.rs shows the latest published crate version and can lag this repository's `main` branch.

## Quick Start

```toml
[dependencies]
domain-check-lib = "0.9.1"
tokio = { version = "1", features = ["full"] }
```

```rust
use domain_check_lib::DomainChecker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let result = checker.check_domain("example.com").await?;

    match result.available {
        Some(true) => println!("{} is AVAILABLE", result.domain),
        Some(false) => println!("{} is TAKEN", result.domain),
        None => println!("{} status is UNKNOWN", result.domain),
    }

    Ok(())
}
```

---

## Key Features

- **Pure Async Rust** — built with tokio for high performance
- **Dual Protocol** — RDAP-first with intelligent WHOIS fallback via IANA server discovery
- **1,200+ TLDs** — universal coverage via IANA bootstrap, with 32 hardcoded TLDs as offline fallback
- **Concurrent Processing** — check multiple domains simultaneously
- **Robust Error Handling** — comprehensive error types with recovery
- **Detailed Information** — extract registrar, dates, and status codes
- **Streaming Support** — real-time results for bulk operations
- **TLD Management** — access preset groups, bootstrap initialization, and domain expansion

---

## Usage Examples

### Bulk Domain Checking

```rust
use domain_check_lib::{DomainChecker, CheckConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CheckConfig::default()
        .with_concurrency(20)
        .with_detailed_info(true);

    let checker = DomainChecker::with_config(config);
    let domains = vec![
        "example.com".to_string(),
        "google.org".to_string(),
        "github.io".to_string(),
    ];

    let results = checker.check_domains(&domains).await?;

    for result in results {
        match result.available {
            Some(true) => println!("{} is available", result.domain),
            Some(false) => println!("{} is taken", result.domain),
            None => println!("{} status unknown", result.domain),
        }
    }

    Ok(())
}
```

### Streaming Results

```rust
use domain_check_lib::DomainChecker;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let domains = vec![
        "example.com".to_string(),
        "startup.org".to_string(),
        "mybrand.net".to_string(),
    ];

    let mut stream = checker.check_domains_stream(&domains);

    while let Some(result) = stream.next().await {
        match result {
            Ok(r) => println!("{}: {:?}", r.domain, r.available),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

### Custom Configuration

```rust
use domain_check_lib::{DomainChecker, CheckConfig};
use std::time::Duration;

let config = CheckConfig::default()
    .with_concurrency(50)                          // Max 50 concurrent checks
    .with_timeout(Duration::from_secs(10))         // 10 second timeout
    .with_whois_fallback(true)                     // Enable WHOIS fallback
    .with_bootstrap(true)                          // Use IANA bootstrap
    .with_detailed_info(true);                     // Extract full domain info

let checker = DomainChecker::with_config(config);
```

### TLD Management & Domain Expansion

```rust
use domain_check_lib::{
    DomainChecker, get_all_known_tlds, get_preset_tlds,
    initialize_bootstrap, expand_domain_inputs,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();

    // Pre-warm the IANA bootstrap cache for full TLD coverage
    initialize_bootstrap().await?;

    // Get all known TLDs (1,200+ after bootstrap, 32 hardcoded offline)
    let all_tlds = get_all_known_tlds();
    println!("Loaded {} TLDs", all_tlds.len());

    // Or use curated presets (11 built-in: startup, popular, tech, creative, etc.)
    let startup_tlds = get_preset_tlds("startup");

    // Expand base names with TLDs
    let domains = expand_domain_inputs(
        &["myapp".to_string(), "mystartup".to_string()],
        &startup_tlds,
    );
    // → myapp.com, myapp.io, myapp.ai, ..., mystartup.com, ...

    let results = checker.check_domains(&domains).await?;
    let available: Vec<_> = results.iter()
        .filter(|r| r.available == Some(true))
        .collect();

    println!("Found {} available domains", available.len());
    Ok(())
}
```

---

## Data Structures

### DomainResult

```rust
pub struct DomainResult {
    pub domain: String,                    // Domain that was checked
    pub available: Option<bool>,           // true = available, false = taken, None = unknown
    pub info: Option<DomainInfo>,          // Detailed registration info
    pub check_duration: Option<Duration>,  // How long the check took
    pub method_used: CheckMethod,          // RDAP, WHOIS, or Bootstrap
    pub error_message: Option<String>,     // Error details (if applicable)
}
```

### DomainInfo

```rust
pub struct DomainInfo {
    pub registrar: Option<String>,
    pub creation_date: Option<String>,
    pub expiration_date: Option<String>,
    pub status: Vec<String>,
    pub updated_date: Option<String>,
    pub nameservers: Vec<String>,
}
```

---

## Error Handling

```rust
use domain_check_lib::DomainCheckError;

match checker.check_domain("invalid-domain").await {
    Ok(result) => println!("Success: {:?}", result),
    Err(DomainCheckError::InvalidDomain { domain, reason }) => {
        eprintln!("Invalid domain '{}': {}", domain, reason);
    }
    Err(DomainCheckError::NetworkError { message, .. }) => {
        eprintln!("Network error: {}", message);
    }
    Err(DomainCheckError::Timeout { operation, duration }) => {
        eprintln!("Timeout after {:?}: {}", duration, operation);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

---

### Bootstrap & WHOIS Discovery

```rust
use domain_check_lib::{initialize_bootstrap, get_whois_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load 1,200+ TLDs from the IANA RDAP bootstrap registry
    initialize_bootstrap().await?;

    // Discover authoritative WHOIS servers for any TLD via IANA referral
    if let Some(server) = get_whois_server("com").await {
        println!(".com WHOIS server: {}", server); // → whois.verisign-grs.com
    }

    Ok(())
}
```

---

## Protocol Support

| Protocol | Role | Details |
|----------|------|---------|
| **RDAP** | Primary | Structured JSON responses, 1,180+ TLDs via IANA bootstrap, rich data |
| **WHOIS** | Fallback | Targeted queries via IANA server discovery, smart parsing, rate limiting |
| **Bootstrap** | Discovery | Bulk IANA registry fetch (enabled by default), 24h cache, zero maintenance |

---

## Related

- **CLI Tool**: [`domain-check`](https://crates.io/crates/domain-check) — command-line interface
- **Repository**: [GitHub](https://github.com/saidutt46/domain-check)
- **Docs Index**: [`docs/README.md`](../docs/README.md)
- **Automation Guide**: [`docs/AUTOMATION.md`](../docs/AUTOMATION.md)
- **FAQ**: [`docs/FAQ.md`](../docs/FAQ.md)

## License

Apache License, Version 2.0 — see the [LICENSE](../LICENSE) file for details.

## Contributing

Contributions welcome. See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for setup and workflow.
