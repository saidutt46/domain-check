# domain-check

Universal domain exploration engine: fast domain availability checks across the internet, as both a CLI and Rust library.

[![Homebrew](https://img.shields.io/badge/Homebrew-available-brightgreen)](https://github.com/saidutt46/homebrew-domain-check)
[![CLI Crate](https://img.shields.io/crates/v/domain-check.svg?label=CLI)](https://crates.io/crates/domain-check)
[![Library Crate](https://img.shields.io/crates/v/domain-check-lib.svg?label=Library)](https://crates.io/crates/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check.svg)](https://crates.io/crates/domain-check)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

<p align="center">
  <img src="./assets/demo.svg" alt="domain-check demo" width="700"/>
</p>

Quick Links: [Installation](#installation) | [Quick Start](#quick-start) | [Why domain-check](#why-domain-check) | [Configuration](#configuration) | [Automation](#automation--ci) | [Presets](#smart-presets) | [Library](#library) | [FAQ](./docs/FAQ.md) | [Contributing](./CONTRIBUTING.md)

## Why domain-check

- **Broad TLD coverage**: check across `1200+` known TLDs with `--all` (bootstrap enabled by default).
- **Dual-protocol engine**: RDAP-first with intelligent WHOIS fallback and IANA-based discovery.
- **Fast and scalable**: concurrent checks up to 100 domains at a time, with streaming or batch output modes.
- **Domain generation built in**: pattern expansion (`\w`, `\d`, `?`), prefix/suffix permutations, dry-run previews.
- **Strong UX for humans**: grouped pretty output, progress indicators, summaries, and detailed domain metadata.
- **Automation-ready output**: JSON/CSV modes, non-TTY-safe behavior, and explicit non-interactive flags.
- **Configurable workflows**: config files, environment variables, custom presets, and deterministic CI setups.
- **CLI + library ecosystem**: same core capabilities available in both `domain-check` and `domain-check-lib`.

## Installation

| Method | Command | Notes |
|---|---|---|
| Homebrew (macOS) | `brew install domain-check` | Easiest install for macOS users |
| Cargo | `cargo install domain-check` | Works on all Rust-supported platforms |
| GitHub Releases | [Download binaries](https://github.com/saidutt46/domain-check/releases) | Prebuilt binaries for macOS/Linux/Windows |

## Quick Start

```bash
# Check one fully qualified domain
domain-check example.com

# Expand base name across TLDs
domain-check mystartup -t com,org,io,dev --batch

# Use curated TLD groups
domain-check myapp --preset startup

# Generate names from a pattern (preview only)
domain-check --pattern "app\d" -t com --dry-run

# Check every known TLD (bootstrap-enabled)
domain-check brand --all --batch
```

Expected pretty output shape:

```text
domain-check v0.9.0 — Checking 8 domains
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

## Core Command Surface

```text
domain-check [OPTIONS] [DOMAINS]...
```

| Category | Key flags |
|---|---|
| Domain selection | `-t, --tld`, `--all`, `--preset`, `--list-presets`, `-f, --file` |
| Generation | `--pattern`, `--prefix`, `--suffix`, `--dry-run`, `-y, --yes` |
| Output | `-p, --pretty`, `-j, --json`, `--csv`, `-i, --info`, `--streaming`, `--batch` |
| Performance/protocol | `-c, --concurrency`, `--no-bootstrap`, `--no-whois`, `-d, --debug` |
| Config | `--config` |

Full reference: [docs/CLI.md](./docs/CLI.md)

## Configuration

Create `domain-check.toml` in your project directory:

```toml
[defaults]
concurrency = 25
preset = "startup"
pretty = true
timeout = "8s"
bootstrap = true

[custom_presets]
my_startup = ["com", "io", "ai", "dev", "app"]

[generation]
prefixes = ["get", "my"]
suffixes = ["hub", "ly"]
```

Config lookup order:
`./domain-check.toml` > `~/.domain-check.toml` > `~/.config/domain-check/config.toml`

Common environment variables:

```bash
DC_CONCURRENCY=50
DC_PRESET=startup
DC_TLD=com,io,dev
DC_PRETTY=true
DC_TIMEOUT=10s
DC_BOOTSTRAP=true
DC_CONFIG=config.toml
DC_FILE=domains.txt
DC_PREFIX=get,my
DC_SUFFIX=hub,ly
```

## Automation & CI

```bash
# Non-interactive structured output
domain-check --file required-domains.txt --json

# Pipe to jq
domain-check --pattern "app\d" -t com --yes --json | jq '.[] | select(.available==true)'

# Stream live results for long runs
domain-check --file large-list.txt --concurrency 75 --streaming
```

Automation guide: [docs/AUTOMATION.md](./docs/AUTOMATION.md)

## Smart Presets

Built-in presets: `startup`, `popular`, `classic`, `enterprise`, `tech`, `creative`, `ecommerce`, `finance`, `web`, `trendy`, `country`.

```bash
domain-check --list-presets
domain-check mybrand --preset creative --pretty
domain-check myshop --preset ecommerce --batch --json
```

## Reliability Notes

- Domain status is network- and registry-dependent. Temporary errors can produce `UNKNOWN` states.
- WHOIS output is less standardized than RDAP; parsing quality varies by registry.
- For repeatable CI workflows, pin behavior with explicit flags (`--batch`, `--json`, `--no-bootstrap`, `--concurrency`).
- docs.rs reflects the latest published crate release and can lag repository `main`.

Troubleshooting and expected edge cases: [docs/FAQ.md](./docs/FAQ.md)

## Library

Use `domain-check-lib` directly in Rust projects:

```toml
[dependencies]
domain-check-lib = "0.9.0"
```

```rust
use domain_check_lib::DomainChecker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let result = checker.check_domain("example.com").await?;
    println!("{} -> {:?}", result.domain, result.available);
    Ok(())
}
```

Library docs: [domain-check-lib/README.md](./domain-check-lib/README.md) | [docs.rs](https://docs.rs/domain-check-lib)

## Project Docs

- CLI reference: [docs/CLI.md](./docs/CLI.md)
- Docs index: [docs/README.md](./docs/README.md)
- Examples and workflows: [docs/EXAMPLES.md](./docs/EXAMPLES.md)
- Automation usage: [docs/AUTOMATION.md](./docs/AUTOMATION.md)
- FAQ: [docs/FAQ.md](./docs/FAQ.md)
- Changelog: [CHANGELOG.md](./CHANGELOG.md)
- Contributing: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Security policy: [SECURITY.md](./SECURITY.md)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](./LICENSE).
