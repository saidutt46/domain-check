# domain-check

**Fast, powerful CLI tool for checking domain availability using RDAP and WHOIS protocols.**

[![Homebrew](https://img.shields.io/badge/Homebrew-available-brightgreen)](https://github.com/saidutt46/homebrew-domain-check)
[![CLI Crate](https://img.shields.io/crates/v/domain-check.svg?label=CLI)](https://crates.io/crates/domain-check)
[![Library Crate](https://img.shields.io/crates/v/domain-check-lib.svg?label=Library)](https://crates.io/crates/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check.svg)](https://crates.io/crates/domain-check)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

<p align="center">
  <img src="./assets/demo.svg" alt="domain-check demo" width="700"/>
</p>

## Key Features

- **Domain Generation** — pattern expansion (`\w`, `\d`, `?`), prefix/suffix permutations, dry-run preview
- **Universal Coverage** — check against all 32 TLDs with `--all` or use smart presets
- **Lightning Fast** — concurrent processing up to 100 domains simultaneously
- **Beautiful Output** — colored results, progress counters, loading spinner, grouped pretty mode
- **Multiple Formats** — pretty terminal display, JSON, CSV for automation, detailed info mode
- **Bulk Processing** — process thousands of domains from files with real-time streaming
- **Agent-Friendly** — `--yes` skips prompts, non-TTY never blocks, `--dry-run` for previews
- **Configuration Files** — persistent settings with TOML configs, env vars, and custom presets
- **Dual Protocol** — RDAP-first with automatic WHOIS fallback for maximum accuracy

---

## Installation

### Homebrew (macOS)
```bash
brew install domain-check
```

### Cargo (All Platforms)
```bash
cargo install domain-check
```

### Pre-built Binaries
Download from [GitHub Releases](https://github.com/saidutt46/domain-check/releases) — available for macOS, Linux, and Windows.

---

## Quick Start

```bash
# Check a single domain
domain-check example.com

# Check multiple TLD variations
domain-check mystartup -t com,org,net,dev --batch

# Generate domains with patterns (dry-run to preview)
domain-check --pattern "app\d" -t com --dry-run
# app0.com, app1.com, ..., app9.com (10 domains)

# Prefix/suffix permutations
domain-check myapp --prefix get,try --suffix hub -t com --dry-run
# getmyapphub.com, getmyapp.com, trymyapphub.com, trymyapp.com, myapphub.com, myapp.com

# Use smart presets (startup, enterprise, country)
domain-check myapp --preset startup

# Pretty mode — grouped results with section headers
domain-check rustcloud --preset startup --pretty --batch
```

**Pretty mode output:**
```
domain-check v0.7.0 — Checking 8 domains
Preset: startup | Concurrency: 20

── Available (3) ──────────────────────────────
  rustcloud.org
  rustcloud.ai
  rustcloud.app

── Taken (5) ──────────────────────────────────
  rustcloud.com
  rustcloud.io
  rustcloud.tech
  rustcloud.dev
  rustcloud.xyz

8 domains in 0.8s  |  3 available  |  5 taken  |  0 unknown
```

---

## Command Reference

```
domain-check [OPTIONS] [DOMAINS]...
```

### Core Options
| Option | Description | Example |
|--------|-------------|---------|
| `-t, --tld <TLD>` | Specify TLDs for base names | `-t com,org,io` |
| `--all` | Check against all 32 known TLDs | `--all` |
| `--preset <NAME>` | Use TLD preset (startup/enterprise/country) | `--preset startup` |
| `-f, --file <FILE>` | Read domains from file | `-f domains.txt` |
| `--config <FILE>` | Use specific config file | `--config my-config.toml` |

### Domain Generation
| Option | Description | Example |
|--------|-------------|---------|
| `--pattern <PAT>` | Generate from pattern (`\w`=letter, `\d`=digit, `?`=either) | `--pattern "app\d"` |
| `--prefix <LIST>` | Prepend prefixes to names | `--prefix get,my,try` |
| `--suffix <LIST>` | Append suffixes to names | `--suffix hub,ly,app` |
| `--dry-run` | Preview generated domains, no network | `--dry-run` |
| `-y, --yes` | Skip confirmation prompts | `--yes` |

### Output Options
| Option | Description |
|--------|-------------|
| `-p, --pretty` | Grouped, structured output with section headers |
| `-j, --json` | JSON format |
| `--csv` | CSV format |
| `-i, --info` | Show detailed domain info (registrar, dates) |
| `--streaming` | Show results as they complete |
| `--batch` | Collect all results before displaying |

### Performance & Protocol
| Option | Description | Default |
|--------|-------------|---------|
| `-c, --concurrency <N>` | Max concurrent checks (1-100) | `20` |
| `-b, --bootstrap` | Use IANA bootstrap for unknown TLDs | `false` |
| `--no-whois` | Disable WHOIS fallback | `false` |
| `-d, --debug` | Show detailed debug information | |

See the full [CLI Reference](./docs/CLI.md) for all options and advanced usage patterns.

---

## Configuration

### Config Files

Create a `.domain-check.toml` to set persistent defaults:

```toml
[defaults]
concurrency = 25
preset = "startup"
pretty = true
timeout = "8s"
bootstrap = true

[custom_presets]
my_startup = ["com", "io", "ai", "dev", "app"]
my_enterprise = ["com", "org", "net", "biz", "info"]

[generation]
prefixes = ["get", "my"]
suffixes = ["hub", "ly"]
```

Config file locations (checked in order):
`./domain-check.toml` > `~/.domain-check.toml` > `~/.config/domain-check/config.toml`

### Environment Variables

```bash
DC_CONCURRENCY=50       # Default concurrency
DC_PRESET=startup       # Default preset
DC_TLD=com,io,dev       # Default TLD list
DC_PRETTY=true          # Enable pretty output
DC_TIMEOUT=10s          # Request timeout
DC_BOOTSTRAP=true       # Enable IANA bootstrap
DC_CONFIG=config.toml   # Config file path
DC_FILE=domains.txt     # Domains file path
DC_PREFIX=get,my        # Default prefixes for generation
DC_SUFFIX=hub,ly        # Default suffixes for generation
```

---

## Examples

### Domain Generation
```bash
# Pattern-based: check all "go0" through "go9" domains
domain-check --pattern "go\d" -t com,io --batch --json

# Prefix/suffix: explore brand variations
domain-check myapp --prefix get,try --suffix hub,ly -t com --pretty --batch

# Dry-run to preview what will be checked
domain-check --pattern "ai\d\d" --prefix cool --preset startup --dry-run

# Agent-friendly: no prompts, structured output
domain-check --pattern "app\d" -t com --yes --json | jq '.[] | select(.available==true)'
```

### Automation & CI/CD
```bash
# Environment-driven configuration
DC_CONCURRENCY=50 DC_PRESET=enterprise domain-check --file domains.txt --json

# Pipe JSON results for downstream processing
domain-check --file required-domains.txt --json | jq '.[] | select(.available)'
```

### Bulk Workflows
```bash
# Domain research pipeline
domain-check --file ideas.txt --preset startup --csv > research.csv

# Brand protection monitoring
domain-check --file brand-variations.txt --all --json > monitoring.json

# High-concurrency processing
domain-check --file large-list.txt --concurrency 75 --streaming
```

### Custom Presets
```bash
# Define presets in .domain-check.toml, then use them
domain-check mystartup --preset my_startup

# Or via environment variable
DC_PRESET=my_startup domain-check mystartup
```

See [Advanced Examples](./docs/EXAMPLES.md) for more enterprise workflows.

---

## Library

Building a Rust app? Use `domain-check-lib` directly:

```toml
[dependencies]
domain-check-lib = "0.8.0"
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
        None => println!("{} is UNKNOWN", result.domain),
    }
    Ok(())
}
```

See the [Library Documentation](https://docs.rs/domain-check-lib) for streaming, bulk processing, and configuration APIs.

---

## Resources

- [CLI Reference & Examples](./docs/CLI.md)
- [Library API Docs](https://docs.rs/domain-check-lib)
- [Advanced Examples](./docs/EXAMPLES.md)
- [Changelog](./CHANGELOG.md)
- [Contributing](./CONTRIBUTING.md)

**Crates:** [domain-check](https://crates.io/crates/domain-check) (CLI) | [domain-check-lib](https://crates.io/crates/domain-check-lib) (Library)

## License

Licensed under the Apache License, Version 2.0 — see the [LICENSE](LICENSE) file for details.
