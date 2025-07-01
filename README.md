# domain-check

**A fast, powerful Rust library and CLI tool for checking domain availability using RDAP and WHOIS protocols.**

[![Crates.io - CLI](https://img.shields.io/crates/v/domain-check.svg)](https://crates.io/crates/domain-check)
[![Crates.io - Library](https://img.shields.io/crates/v/domain-check-lib.svg)](https://crates.io/crates/domain-check-lib)
[![Documentation](https://docs.rs/domain-check-lib/badge.svg)](https://docs.rs/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check.svg)](https://crates.io/crates/domain-check)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/saidutt46/domain-check/workflows/CI/badge.svg)](https://github.com/saidutt46/domain-check/actions)
[![Security Audit](https://github.com/saidutt46/domain-check/workflows/Security%20Audit/badge.svg)](https://github.com/saidutt46/domain-check/security)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

<!-- Demos -->
![Basic Usage](./assets/demo.svg)
![Bulk File Demo](./assets/file-demo.svg)

---

## 🚀 Overview

**domain-check** is a high-performance Rust toolkit for accurate domain availability checks.  
It combines a robust asynchronous library with a flexible CLI, optimized for RDAP with seamless WHOIS fallback. Ideal for developers, DevOps workflows, domain investors, and automation scripts. 

## ✨ Features

- ✅ **Rust-native async library** (`domain-check-lib`) built on tokio for high concurrency
- 🌐 **Dual protocol support**: RDAP-first with smart WHOIS fallback for maximum TLD coverage
- 🚀 **Bulk domain checking** with powerful concurrency, connection pooling, and rate limiting
- 📁 **Advanced file input support**: check hundreds or thousands of domains from a file, with comments and inline TLD expansion
- 🔄 **Streaming results mode**: see each domain result as it completes — ideal for large lists
- 📊 **Flexible output formats**: JSON for integrations, CSV for spreadsheets, or colorful human-readable output
- 🎯 **Smart TLD expansion**: specify TLDs with `-t` to auto-expand base names like `startup` into `startup.com`, `startup.io`, etc.
- 🔍 **Detailed info mode**: includes registrar, creation & expiry dates, status, nameservers
- ⚙️ **Highly configurable**: set concurrency up to 100, customize timeouts, enable or disable WHOIS fallback, or use IANA bootstrap for unknown TLDs
- 🐛 **Debug & verbose modes**: get detailed protocol-level logs and error diagnostics with `--debug` and `--verbose`
- 📝 **Perfect for CI/CD & shell scripts**: pipe JSON or CSV directly to tools like `jq` or `grep`
- 🌐 **Universal TLD checking with --all flag** (35+ TLDs)
- 🎯 **Smart TLD presets**: startup, enterprise, country presets for common scenarios
- 📊 **Enhanced error reporting** with intelligent error aggregation and actionable summaries
- ⚡ **No artificial limits** - check as many domains as you need
- 🔄 **Auto-bootstrap** - automatically enables comprehensive registry coverage
- 💻 **Robust CLI options**:
  - `--file <domains.txt>` to load domains
  - `--streaming` for real-time feedback
  - `--batch` to collect all results before printing
  - `--force` to override safety limits for massive checks
  - `--no-whois` to disable fallback
  - `--bootstrap` for dynamic RDAP discovery

---

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [Installation](#-installation)
- [CLI Usage](#-cli-usage)
  - [Basic Examples](#basic-examples)
  - [Advanced Examples](#advanced-examples)
  - [Command Reference](#command-reference)
- [Library Usage](#-library-usage)
- [Features](#-features)
- [Performance](#-performance)
- [Comparison](#-comparison)
- [Contributing](#-contributing)

---

## 🚀 Quick Start

### CLI Quick Start

```bash
# Install
cargo install domain-check

# Check single domain
domain-check example.com

# Check multiple TLDs
domain-check startup -t com,org,net,io

# NEW in v0.5.0: Check ALL known TLDs at once
domain-check myapp --all

# NEW: Use curated TLD presets  
domain-check startup -t com,org,net,io    # OLD way
domain-check startup --preset startup     # NEW way (same result!)

# NEW: Enterprise-focused domains
domain-check mybrand --preset enterprise

# NEW: Country code domains
domain-check mysite --preset country

# Bulk check from file
domain-check --file domains.txt -t com,org
```

### Library Quick Start

```toml
[dependencies]
domain-check-lib = "0.5.0"
tokio = { version = "1", features = ["full"] }
```

```rust
use domain_check_lib::DomainChecker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let result = checker.check_domain("example.com").await?;
    println!("Available: {:?}", result.available);
    Ok(())
}

// NEW in v0.5.0: Use TLD presets in your applications
use domain_check_lib::{DomainChecker, get_preset_tlds, get_all_known_tlds};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    
    // Check against startup-focused TLDs
    let startup_tlds = get_preset_tlds("startup").unwrap();
    let expanded_domains = domain_check_lib::expand_domain_inputs(
        &["myapp".to_string()], 
        &Some(startup_tlds)
    );
    
    let results = checker.check_domains(&expanded_domains).await?;
    println!("Checked {} startup domains", results.len());
    
    Ok(())
}
```

---

## 📦 Installation

### Installing the CLI

#### Via Cargo (Recommended)

```bash
cargo install domain-check
```

#### Pre-built Binaries

Download pre-built binaries for your platform from [GitHub Releases](https://github.com/saidutt46/domain-check/releases).

```bash
# macOS
curl -LO https://github.com/saidutt46/domain-check/releases/latest/download/domain-check-macos-aarch64.tar.gz
tar -xzf domain-check-macos-aarch64.tar.gz
sudo mv domain-check /usr/local/bin/

# Linux
curl -LO https://github.com/saidutt46/domain-check/releases/latest/download/domain-check-linux-x86_64.tar.gz
tar -xzf domain-check-linux-x86_64.tar.gz
sudo mv domain-check /usr/local/bin/

# Windows
# Download domain-check-windows-x86_64.zip from releases
# Extract and add to PATH
```

#### From Source

```bash
git clone https://github.com/saidutt46/domain-check.git
cd domain-check
cargo install --path domain-check
```

---

## 🖥️ CLI Usage

### Basic Examples

#### Check a single domain

```bash
domain-check example.com
```
Output:
```
🔴 example.com is TAKEN
```

#### Check domain across multiple TLDs

```bash
domain-check myawesome -t com,net,org,io
```
Output:
```
🔍 Checking 4 domains with concurrency: 10

🔴 myawesome.com is TAKEN
🟢 myawesome.net is AVAILABLE
🟢 myawesome.org is AVAILABLE
🔴 myawesome.io is TAKEN

✅ 4 domains processed in 1.2s: 🟢 2 available, 🔴 2 taken, ⚠️ 0 unknown
```

#### 🆕 Universal Domain Checking

```bash
# Check against ALL known TLDs (~42 TLDs)
domain-check myapp --all

# Real-time results as they complete
domain-check myapp --all --streaming
```

#### 🆕 Smart TLD Presets

```bash
# Startup/tech-focused TLDs (8 TLDs)
domain-check myapp --preset startup
# Checks: .com, .org, .io, .ai, .tech, .app, .dev, .xyz

# Enterprise/business TLDs (6 TLDs)  
domain-check mybrand --preset enterprise
# Checks: .com, .org, .net, .info, .biz, .us

# Major country codes (9 TLDs)
domain-check mysite --preset country
# Checks: .us, .uk, .de, .fr, .ca, .au, .jp, .br, .in
```

#### Check with detailed information

```bash
domain-check google.com --info
```
Output:
```
🔴 google.com is TAKEN (Registrar: MarkMonitor Inc., Created: 1997-09-15, Expires: 2028-09-14)
```

### Advanced Examples

#### Bulk domain checking from file

Create a file `domains.txt`:
```text
# Startup ideas
unicorn-startup
my-awesome-app
cool-domain

# Existing domains to verify
google.com
github.com
```

Run bulk check:
```bash
domain-check --file domains.txt -t com,io --concurrency 20
```

Output:
```
🔍 Checking 10 domains with concurrency: 20

🟢 unicorn-startup.com is AVAILABLE
🟢 unicorn-startup.io is AVAILABLE
🟢 my-awesome-app.com is AVAILABLE
🔴 cool-domain.com is TAKEN
🔴 google.com is TAKEN
🔴 github.com is TAKEN
...

✅ 10 domains processed in 0.8s: 🟢 3 available, 🔴 7 taken, ⚠️ 0 unknown
```

#### JSON output for scripting

```bash
# Get available domains as JSON
domain-check startup -t com,org,net --json | jq '.[] | select(.available==true) | .domain'
```

Output:
```json
[
  {
    "domain": "startup.com",
    "available": false,
    "method_used": "rdap",
    "check_duration": { "secs": 0, "nanos": 234567890 }
  },
  {
    "domain": "startup.org",
    "available": true,
    "method_used": "rdap",
    "check_duration": { "secs": 0, "nanos": 123456789 }
  }
]
```

#### CSV output for spreadsheets

```bash
domain-check example startup -t com,org --csv > results.csv
```

Output:
```csv
domain,available,registrar,created,expires,method
example.com,false,Example Inc.,1995-08-14,2025-08-13,rdap
example.org,false,PIR,2000-01-01,2025-01-01,rdap
startup.com,false,-,-,-,rdap
startup.org,true,-,-,-,rdap
```

#### Bootstrap for unknown TLDs

```bash
# Check rare or new TLDs with IANA bootstrap
domain-check example.restaurant --bootstrap --debug
```

Output:
```
🔍 No known RDAP endpoint for .restaurant, trying bootstrap registry...
🔍 Found endpoint: https://rdap.donuts.co/domain/
🔴 example.restaurant is TAKEN
```

#### Streaming mode for real-time results

```bash
# See results as they complete (great for large batches)
domain-check --file large-list.txt --streaming
```

<!-- Screenshot suggestion: Add animated GIF here showing streaming results -->

#### Force batch mode for sorted output

```bash
# Collect all results before displaying
domain-check --file domains.txt --batch --pretty
```

### 🆕 Powerful Bulk Operations

```bash
# Check startup names against all TLDs
echo -e "airbnb\nuber\nstripe" > startups.txt
domain-check --file startups.txt --all --streaming

# Enterprise domain audit
domain-check --file companies.txt --preset enterprise --csv > audit.csv
```

### Command Reference

```
USAGE:
    domain-check [OPTIONS] [DOMAINS]...

ARGS:
    <DOMAINS>...    Domain names to check (supports both base names and FQDNs)

OPTIONS:
    -t, --tld <TLD>              TLDs to check for base domain names (comma-separated or multiple -t flags)
    -f, --file <FILE>            Input file with domains to check (one per line)
    -c, --concurrency <N>        Max concurrent domain checks (default: 10, max: 100)
        --force                  Override the 500 domain limit for bulk operations
    -i, --info                   Show detailed domain information when available
    -b, --bootstrap              Use IANA bootstrap to find RDAP endpoints for unknown TLDs
        --no-whois               Disable automatic WHOIS fallback
        --all                    Check against all known TLDs (~42 TLDs)
        --preset <NAME>          Use TLD preset: startup, enterprise, country
                                 startup (8): com, org, io, ai, tech, app, dev, xyz
                                 enterprise (6): com, org, net, info, biz, us  
                                 country (9): us, uk, de, fr, ca, au, jp, br, in
    -j, --json                   Output results in JSON format
        --csv                    Output results in CSV format
    -p, --pretty                 Enable colorful, formatted output
        --batch                  Force batch mode - collect all results before displaying
        --streaming              Force streaming mode - show results as they complete
    -d, --debug                  Show detailed debug information and error messages
    -v, --verbose                Enable verbose logging
    -h, --help                   Print help information
    -V, --version                Print version information
```

### Usage Patterns

#### 🎯 Domain Investor Workflow

```bash
# Generate domain ideas and check availability
echo "ai-startup
ml-platform
data-wizard" > ideas.txt

domain-check --file ideas.txt -t com,io,ai --info --csv > portfolio.csv
```

#### 🚀 Startup Name Search

```bash
# Check your startup name across all important TLDs
domain-check mycoolstartup -t com,io,co,app,dev,tech,xyz
```

#### 🔍 Brand Protection Check

```bash
# Monitor brand variations
echo "mybrand
my-brand
mybrand-app
mybrand-official" > brand-check.txt

domain-check --file brand-check.txt -t com,net,org,io --json > brand-report.json
```

#### ⚡ High-Performance Bulk Checking

```bash
# Process large lists efficiently
domain-check --file thousand-domains.txt \
  --concurrency 50 \
  --force \
  --streaming \
  --csv > results.csv
```

### Tips and Tricks

1. **Smart TLD Expansion**: Base names (no dots) automatically expand with specified TLDs
   ```bash
   domain-check startup -t com,org  # Checks: startup.com, startup.org
   domain-check startup.com -t org  # Checks: startup.com only (FQDN not expanded)
   ```

2. **File Format Flexibility**: Domain files support comments and mixed formats
   ```text
   # Premium domains
   example
   test.com      # Already includes TLD
   
   # Ideas for later
   my-app        # Will use TLDs from -t flag
   ```

3. **Pipe-Friendly Output**: Use with other Unix tools
   ```bash
   # Find all available .com domains
   domain-check --file list.txt -t com --json | \
     jq -r '.[] | select(.available==true) | .domain' | \
     grep '\.com
   ```

4. **Performance Tuning**: Adjust concurrency based on domain count
   - 1-50 domains: Default (10) is fine
   - 50-200 domains: Use `--concurrency 25`
   - 200+ domains: Use `--concurrency 50` with `--streaming`

---

## 📚 Library Usage

For detailed library documentation, see our [Library Usage Guide](https://docs.rs/domain-check-lib).

### Quick Examples

```rust
use domain_check_lib::{DomainChecker, CheckConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the checker
    let config = CheckConfig::default()
        .with_concurrency(20)
        .with_timeout(Duration::from_secs(10))
        .with_detailed_info(true);
    
    let checker = DomainChecker::with_config(config);
    
    // Check multiple domains
    let domains = vec!["example.com", "test.org", "startup.io"];
    let results = checker.check_domains(&domains).await?;
    
    for result in results {
        println!("{}: {:?}", result.domain, result.available);
    }
    
    Ok(())
}
```

---

### Supported TLDs

40+ built-in TLD mappings including:
- **Generic**: com, net, org, info, biz
- **Tech**: io, app, dev, tech, ai
- **Countries**: us, uk, de, fr, jp, au, ca
- **New**: xyz, club, online, site, blog

Additional TLDs supported via IANA bootstrap with `--bootstrap` flag.

---

## ⚡ Performance

Domain Check v0.4.0 delivers exceptional performance through:

- **Smart Concurrency**: Configurable parallel processing (up to 100 concurrent checks)
- **Connection Pooling**: Reuses HTTP connections for faster subsequent requests
- **Optimized Timeouts**: Registry-specific timeout tuning
- **Streaming Architecture**: Results available immediately as they complete

---

## 🔍 Comparison

| Feature | domain-check | whois-cli | DNSLookup |
|---------|--------------|-----------|-----------|
| RDAP Protocol | ✅ | ❌ | ❌ |
| WHOIS Fallback | ✅ | ✅ | ❌ |
| Bulk Checking | ✅ | ❌ | ❌ |
| Concurrent Processing | ✅ | ❌ | ❌ |
| JSON/CSV Output | ✅ | ❌ | Limited |
| Library + CLI | ✅ | ❌ | ❌ |
| Streaming Results | ✅ | ❌ | ❌ |
| TLD Expansion | ✅ | ❌ | ❌ |
| Bootstrap Registry | ✅ | ❌ | ❌ |

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

```bash
# Setup
git clone https://github.com/saidutt46/domain-check.git
cd domain-check
cargo build

# Run tests
cargo test

# Run with local changes
cargo run -- example.com
```

---

## 📝 License

Licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🔗 Links

- **Documentation**: [docs.rs/domain-check-lib](https://docs.rs/domain-check-lib)
- **Crate (Library)**: [crates.io/crates/domain-check-lib](https://crates.io/crates/domain-check-lib)
- **Crate (CLI)**: [crates.io/crates/domain-check](https://crates.io/crates/domain-check)
- **Repository**: [github.com/saidutt46/domain-check](https://github.com/saidutt46/domain-check)

---

*Built with ❤️ in Rust :: GVS*