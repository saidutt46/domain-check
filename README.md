# domain-check

Universal domain exploration engine: fast domain availability checks across the internet — as a CLI, Rust library, and MCP server for AI agents.

[![Homebrew](https://img.shields.io/badge/Homebrew-available-brightgreen)](https://github.com/saidutt46/homebrew-domain-check)
[![CLI Crate](https://img.shields.io/crates/v/domain-check.svg?label=CLI)](https://crates.io/crates/domain-check)
[![Library Crate](https://img.shields.io/crates/v/domain-check-lib.svg?label=Library)](https://crates.io/crates/domain-check-lib)
[![MCP Server](https://img.shields.io/crates/v/domain-check-mcp.svg?label=MCP)](https://crates.io/crates/domain-check-mcp)
[![Downloads](https://img.shields.io/crates/d/domain-check.svg)](https://crates.io/crates/domain-check)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)

<p align="center">
  <img src="https://raw.githubusercontent.com/saidutt46/domain-check/main/assets/demo.svg" alt="domain-check demo" width="700"/>
</p>

Quick Links: [Installation](#installation) | [Quick Start](#quick-start) | [Use Cases](#use-cases) | [Output Formats](#output-formats) | [Presets](#smart-presets) | [Configuration](#configuration) | [Automation](#automation--ci) | [Library](#library) | [MCP Server](#mcp-server) | [FAQ](https://github.com/saidutt46/domain-check/blob/main/docs/FAQ.md) | [Contributing](https://github.com/saidutt46/domain-check/blob/main/CONTRIBUTING.md)

## Why domain-check

- **1,200+ TLDs out of the box** — IANA bootstrap loads the full registry automatically. No config needed. 32 hardcoded TLDs work offline as fallback.
- **Dual-protocol engine** — RDAP-first with automatic WHOIS fallback. IANA server discovery covers ~189 ccTLDs that lack RDAP (`.es`, `.co`, `.eu`, `.jp`).
- **Fast** — up to 100 concurrent checks, streaming results as they complete. 2.7 MB release binary.
- **Domain generation** — pattern expansion (`\w`=letter, `\d`=digit, `?`=either), prefix/suffix permutations, and `--dry-run` to preview before checking.
- **11 curated presets** — `startup`, `tech`, `creative`, `finance`, `ecommerce`, and more. Or define your own in config.
- **Rich output** — grouped pretty display, JSON, CSV. Registrar info, creation/expiration dates, and status codes with `--info`.
- **CI and automation friendly** — `--json`/`--csv` to stdout, `--yes` to skip prompts, `--force` for large runs, automatic non-TTY detection.
- **Configurable** — TOML config files, `DC_*` environment variables, custom presets, and clear precedence rules.
- **Agent-native** — MCP server exposes all tools to AI coding agents (Claude, Codex, Gemini, Cursor, VS Code Copilot, and any MCP client).
- **CLI + library + MCP** — same engine powers the CLI, [`domain-check-lib`](https://crates.io/crates/domain-check-lib) (Rust library), and [`domain-check-mcp`](https://crates.io/crates/domain-check-mcp) (MCP server).

## Installation

| Method | Command | Notes |
|---|---|---|
| Homebrew (macOS) | `brew install domain-check` | Easiest install for macOS users |
| Cargo (CLI) | `cargo install domain-check` | Works on all Rust-supported platforms |
| Cargo (MCP) | `cargo install domain-check-mcp` | MCP server for AI agents |
| GitHub Releases | [Download binaries](https://github.com/saidutt46/domain-check/releases) | CLI + MCP binaries for macOS, Linux, Windows |

## Quick Start

```bash
# Check a single domain
domain-check example.com

# Expand a base name across TLDs
domain-check mystartup -t com,org,io,dev

# Use a curated preset
domain-check myapp --preset startup --pretty

# Generate names with a pattern (preview only)
domain-check --pattern "app\d" -t com --dry-run

# Add prefixes and suffixes
domain-check myapp --prefix get,try --suffix hub,ly -t com,io

# Get registrar and date info
domain-check target.com --info

# Check every known TLD
domain-check brand --all --batch
```

Pretty output:

```text
domain-check v0.9.1 — Checking 8 domains
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

## Use Cases

```bash
# Startup naming — scan tech TLDs for your brand
domain-check coolname --preset startup --pretty

# Brand protection — audit every TLD for your trademark
domain-check mybrand --all --json > audit.json

# Pre-purchase validation — check registrar and expiry before buying
domain-check target.com --info

# Bulk pipeline — feed a list, export results
domain-check --file ideas.txt --preset tech --csv > results.csv

# Name generation — explore prefix/suffix combos
domain-check app --prefix get,my,try --suffix hub,ly -t com,io --dry-run
```

## Output Formats

**Default** — one line per domain, colored status:

```text
myapp.com TAKEN
myapp.io AVAILABLE
myapp.dev TAKEN
```

**Pretty** (`--pretty`) — grouped by status with summary:

```text
── Available (1) ──────────────────────────────
  myapp.io

── Taken (2) ──────────────────────────────────
  myapp.com
  myapp.dev

3 domains in 0.4s  |  1 available  |  2 taken  |  0 unknown
```

**JSON** (`--json`) — structured, pipe to `jq`:

```json
[
  {
    "domain": "myapp.com",
    "available": false,
    "method": "RDAP"
  },
  {
    "domain": "myapp.io",
    "available": true,
    "method": "RDAP"
  }
]
```

**CSV** (`--csv`) — import into spreadsheets or databases:

```text
domain,status,method
myapp.com,TAKEN,RDAP
myapp.io,AVAILABLE,RDAP
```

**Info** (`--info`) — registrar, dates, and status codes:

```text
myapp.com TAKEN
  Registrar: Example Registrar, Inc.
  Created: 2015-03-12  Expires: 2026-03-12
  Status: clientTransferProhibited
```

Full reference: [docs/CLI.md](https://github.com/saidutt46/domain-check/blob/main/docs/CLI.md)

## Smart Presets

11 built-in presets covering common domains strategies:

| Preset | TLDs | Use case |
|---|---|---|
| `startup` | com, org, io, ai, tech, app, dev, xyz | Tech startups |
| `popular` | com, net, org, io, ai, app, dev, tech, me, co, xyz | General coverage |
| `classic` | com, net, org, info, biz | Traditional gTLDs |
| `enterprise` | com, org, net, info, biz, us | Business and government |
| `tech` | io, ai, app, dev, tech, cloud, software, + 5 more | Developer tools |
| `creative` | design, art, studio, media, photography, + 5 more | Artists and media |
| `ecommerce` | shop, store, market, sale, deals, + 3 more | Online retail |
| `finance` | finance, capital, fund, money, investments, + 4 more | Fintech |
| `web` | web, site, website, online, blog, page, + 3 more | Web services |
| `trendy` | xyz, online, site, top, icu, fun, space, + 6 more | New gTLDs |
| `country` | us, uk, de, fr, ca, au, br, in, nl | International |

```bash
domain-check --list-presets                          # See all presets with full TLD lists
domain-check mybrand --preset creative --pretty      # Use a preset
```

Define custom presets in your config file:

```toml
[custom_presets]
my_stack = ["com", "io", "dev", "app"]
```

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
DC_CONCURRENCY=50    DC_PRESET=startup    DC_TLD=com,io,dev
DC_PRETTY=true       DC_TIMEOUT=10s       DC_BOOTSTRAP=true
DC_PREFIX=get,my     DC_SUFFIX=hub,ly     DC_FILE=domains.txt
```

## Automation & CI

```bash
# Non-interactive structured output
domain-check --file required-domains.txt --json

# Pipe to jq
domain-check --pattern "app\d" -t com --yes --json | jq '.[] | select(.available==true)'

# Stream live results for long runs
domain-check --file large-list.txt --concurrency 75 --streaming

# Large batch with no prompts
domain-check --file huge-list.txt --all --force --yes --csv > results.csv
```

CI-friendly behavior:
- `--yes` / `--force` skip all confirmation prompts
- Non-TTY environments (piped, CI) never prompt — scripts are never blocked
- Spinner writes to stderr; stdout stays clean for piping
- `--no-bootstrap` for deterministic, offline-safe checks against 32 hardcoded TLDs

Automation guide: [docs/AUTOMATION.md](https://github.com/saidutt46/domain-check/blob/main/docs/AUTOMATION.md)

## Library

Use `domain-check-lib` directly in Rust projects:

```toml
[dependencies]
domain-check-lib = "1.0.1"
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

Library docs: [domain-check-lib/README.md](https://github.com/saidutt46/domain-check/blob/main/domain-check-lib/README.md) | [docs.rs](https://docs.rs/domain-check-lib)

## MCP Server

`domain-check-mcp` exposes domain checking as tools for AI coding agents via the [Model Context Protocol](https://modelcontextprotocol.io). Works with any MCP-compatible client.

```bash
# Install
cargo install domain-check-mcp

# Add to your agent (Claude Code example — works similarly for Codex, Gemini CLI, etc.)
claude mcp add domain-check -- domain-check-mcp
```

6 tools available: `check_domain`, `check_domains`, `check_with_preset`, `generate_names`, `list_presets`, `domain_info`.

Then ask your agent naturally: *"Is coolstartup.com available?"* or *"Check mybrand across the startup preset"*.

Supports: Claude Code, Claude Desktop, VS Code Copilot, Cursor, Windsurf, JetBrains, OpenAI Codex CLI, Gemini CLI, and any MCP stdio client.

Full setup: [domain-check-mcp/README.md](https://github.com/saidutt46/domain-check/blob/main/domain-check-mcp/README.md)

## Reliability Notes

- Domain status is network- and registry-dependent. Temporary errors can produce `UNKNOWN` states.
- WHOIS output is less standardized than RDAP; parsing quality varies by registry.
- For repeatable CI workflows, pin behavior with explicit flags (`--batch`, `--json`, `--no-bootstrap`, `--concurrency`).

Troubleshooting and expected edge cases: [docs/FAQ.md](https://github.com/saidutt46/domain-check/blob/main/docs/FAQ.md)

## Project Docs

- CLI reference: [docs/CLI.md](https://github.com/saidutt46/domain-check/blob/main/docs/CLI.md)
- Library: [domain-check-lib/README.md](https://github.com/saidutt46/domain-check/blob/main/domain-check-lib/README.md) | [docs.rs](https://docs.rs/domain-check-lib)
- MCP server: [domain-check-mcp/README.md](https://github.com/saidutt46/domain-check/blob/main/domain-check-mcp/README.md)
- Examples and workflows: [docs/EXAMPLES.md](https://github.com/saidutt46/domain-check/blob/main/docs/EXAMPLES.md)
- Automation usage: [docs/AUTOMATION.md](https://github.com/saidutt46/domain-check/blob/main/docs/AUTOMATION.md)
- FAQ: [docs/FAQ.md](https://github.com/saidutt46/domain-check/blob/main/docs/FAQ.md)
- Changelog: [CHANGELOG.md](https://github.com/saidutt46/domain-check/blob/main/CHANGELOG.md)
- Contributing: [CONTRIBUTING.md](https://github.com/saidutt46/domain-check/blob/main/CONTRIBUTING.md)
- Security policy: [SECURITY.md](https://github.com/saidutt46/domain-check/blob/main/SECURITY.md)

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/saidutt46/domain-check/blob/main/LICENSE-APACHE)
- [MIT License](https://github.com/saidutt46/domain-check/blob/main/LICENSE-MIT)

at your option.
