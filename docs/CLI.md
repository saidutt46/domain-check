# CLI Reference & Examples

Complete guide to using the `domain-check` command-line tool.

Related docs: [README](../README.md) | [Automation Guide](./AUTOMATION.md) | [FAQ](./FAQ.md) | [Examples](./EXAMPLES.md)

## Table of Contents

- [Basic Usage](#basic-usage)
- [Configuration Files](#%EF%B8%8F-configuration-files)
- [Environment Variables](#-environment-variables)
- [Command Reference](#command-reference)
- [TLD Options](#tld-options)
- [Custom Presets](#-custom-presets)
- [Output Formats](#output-formats)
- [File Processing](#file-processing)
- [Performance & Concurrency](#performance--concurrency)
- [Advanced Features](#advanced-features)
- [Tips & Tricks](#tips--tricks)
- [Example Workflows](#example-workflows)

---

## Basic Usage

### Single Domain Check
```bash
# Default output — colored status
domain-check example.com
# example.com TAKEN

# Pretty mode — grouped layout with sections
domain-check example.com --pretty
#   example.com                    TAKEN
```

### Multiple Domain Arguments
```bash
# Check multiple domains at once
domain-check example.com google.com startup.org
# example.com TAKEN
# google.com TAKEN
# startup.org AVAILABLE
#
# 3 domains in 0.2s  |  1 available  |  2 taken  |  0 unknown
```

---

## ⚙️ Configuration Files

### Configuration File Support

Domain-check supports persistent configuration through TOML files. This eliminates repetitive typing and enables team standardization.

#### Configuration File Locations (checked in order)

1. `./domain-check.toml` (project-specific)
2. `~/.domain-check.toml` (user global)
3. `~/.config/domain-check/config.toml` (XDG standard)

#### Basic Configuration Example

```toml
# domain-check.toml
[defaults]
concurrency = 25
preset = "startup"
pretty = true
timeout = "8s"
bootstrap = true        # enabled by default; set false to disable

[custom_presets]
my_startup = ["com", "io", "ai", "dev", "app"]
my_enterprise = ["com", "org", "net", "biz", "info"]

[generation]
prefixes = ["get", "my"]
suffixes = ["hub", "ly"]

[output]
default_format = "pretty"
csv_headers = true
```

#### Usage with Configuration

```bash
# Uses settings from config file automatically
domain-check mystartup

# Override specific settings
domain-check mystartup --concurrency 50

# Use specific config file
domain-check --config my-team-config.toml mystartup

# Use custom preset from config
domain-check mystartup --preset my_startup
```

### Precedence Rules

Settings are resolved in this order (highest to lowest):
1. **CLI arguments** (explicit user input)
2. **Environment variables** (DC_*)
3. **Local config** (./domain-check.toml, or ./.domain-check.toml)
4. **Global config** (~/.domain-check.toml)
5. **XDG config** (~/.config/domain-check/config.toml)
6. **Built-in defaults**

---

## 🔧 Environment Variables

### Complete Environment Variable Support

All CLI options can be set via environment variables using the `DC_*` prefix:

| Environment Variable | CLI Equivalent | Example | Description |
|---------------------|----------------|---------|-------------|
| `DC_CONCURRENCY` | `--concurrency` | `DC_CONCURRENCY=50` | Max concurrent checks |
| `DC_PRESET` | `--preset` | `DC_PRESET=startup` | Default TLD preset |
| `DC_TLD` | `--tld` | `DC_TLD=com,io,dev` | Default TLD list |
| `DC_PRETTY` | `--pretty` | `DC_PRETTY=true` | Enable pretty output |
| `DC_TIMEOUT` | N/A | `DC_TIMEOUT=10s` | Request timeout |
| `DC_BOOTSTRAP` | (enabled by default) | `DC_BOOTSTRAP=false` | Enable/disable IANA bootstrap |
| `DC_WHOIS_FALLBACK` | `--no-whois` | `DC_WHOIS_FALLBACK=false` | WHOIS fallback |
| `DC_DETAILED_INFO` | `--info` | `DC_DETAILED_INFO=true` | Detailed domain info |
| `DC_JSON` | `--json` | `DC_JSON=true` | JSON output format |
| `DC_CSV` | `--csv` | `DC_CSV=true` | CSV output format |
| `DC_FILE` | `--file` | `DC_FILE=domains.txt` | Default domains file |
| `DC_CONFIG` | `--config` | `DC_CONFIG=my-config.toml` | Default config file |
| `DC_PREFIX` | `--prefix` | `DC_PREFIX=get,my` | Default prefixes |
| `DC_SUFFIX` | `--suffix` | `DC_SUFFIX=hub,ly` | Default suffixes |

### Environment Variable Examples

```bash
# CI/CD Pipeline
DC_CONCURRENCY=30 DC_PRESET=enterprise domain-check --file domains.txt --json

# Docker Container
docker run -e DC_PRESET=startup -e DC_PRETTY=true domain-check:latest myapp

# Development Environment
export DC_CONCURRENCY=25
export DC_PRESET=startup
export DC_PRETTY=true
# Now all commands use these defaults

# Team Standardization
DC_CONFIG=team-config.toml domain-check mystartup
```

---

## Command Reference

### Core Options

| Flag | Description | Example |
|------|-------------|---------|
| `<DOMAINS>...` | Domain names to check | `domain-check example.com google.com` |
| `-t, --tld <TLD>` | Specify TLDs for base names | `domain-check startup -t com,org,io` |
| `--all` | Check against all known TLDs (1,200+ with bootstrap) | `domain-check myapp --all` |
| `--preset <NAME>` | Use TLD preset (11 built-in or custom) | `domain-check myapp --preset startup` |
| `--list-presets` | List all available TLD presets and exit | `domain-check --list-presets` |
| `-f, --file <FILE>` | Read domains from file | `domain-check --file domains.txt` |
| `--config <FILE>` | Use specific config file | `domain-check --config my-config.toml` |
| `-h, --help` | Show help information | `domain-check --help` |
| `-V, --version` | Show version | `domain-check --version` |

### TLD Selection

| Flag | Description | Example |
|------|-------------|---------|
| `-t, --tld <TLD>` | Specify TLDs for base names | `domain-check startup -t com,org,io` |
| `--all` | Check against all known TLDs (1,200+ with bootstrap) | `domain-check myapp --all` |
| `--preset <NAME>` | Use TLD preset (11 built-in or custom) | `domain-check myapp --preset startup` |
| `--list-presets` | List all available TLD presets and exit | `domain-check --list-presets` |

### Input Sources

| Flag | Description | Example |
|------|-------------|---------|
| `-f, --file <FILE>` | Read domains from file | `domain-check --file domains.txt` |
| `--pattern <PAT>` | Generate names from pattern | `domain-check --pattern "test\d"` |
| `--prefix <LIST>` | Prepend prefixes to names | `domain-check app --prefix get,my` |
| `--suffix <LIST>` | Append suffixes to names | `domain-check app --suffix hub,ly` |
| `--dry-run` | Preview domains without checking | `domain-check --pattern "x\d" --dry-run` |
| `-y, --yes` | Skip confirmation prompts | `domain-check --pattern "x\d\d" --yes` |

### Output Control

| Flag | Description | Example |
|------|-------------|---------|
| `-j, --json` | Output in JSON format | `domain-check example.com --json` |
| `--csv` | Output in CSV format | `domain-check example.com --csv` |
| `-p, --pretty` | Grouped, structured output with section headers | `domain-check example.com --pretty` |
| `-i, --info` | Show detailed domain information | `domain-check example.com --info` |

### Processing Modes

| Flag | Description | Example |
|------|-------------|---------|
| `--streaming` | Show results as they complete | `domain-check --file large.txt --streaming` |
| `--batch` | Collect all results before showing | `domain-check --file domains.txt --batch` |

### Performance

| Flag | Description | Example |
|------|-------------|---------|
| `-c, --concurrency <N>` | Max concurrent checks (1-100) | `domain-check --file domains.txt -c 50` |
| `--force` | Override safety limits | `domain-check --file huge.txt --force` |

**Default concurrency:** 20

### Protocol Options

| Flag | Description | Example |
|------|-------------|---------|
| `--no-bootstrap` | Disable IANA bootstrap (use only 32 hardcoded TLDs) | `domain-check myapp --all --no-bootstrap` |
| `--no-whois` | Disable WHOIS fallback | `domain-check example.com --no-whois` |

Bootstrap is enabled by default. It fetches the full IANA RDAP registry (~1,180 TLDs) on first use and caches it for 24 hours. For TLDs without RDAP, the WHOIS fallback automatically discovers the authoritative WHOIS server via IANA referral.

### Debugging

| Flag | Description | Example |
|------|-------------|---------|
| `-d, --debug` | Show detailed debug information | `domain-check example.com --debug` |
| `-v, --verbose` | Enable verbose logging | `domain-check example.com --verbose` |

---

## TLD Options

### Manual TLD Specification
```bash
# Single TLD
domain-check startup -t com
# startup.com TAKEN

# Multiple TLDs (comma-separated)
domain-check startup -t com,org,io,ai
# startup.com TAKEN
# startup.org AVAILABLE
# startup.io TAKEN  
# startup.ai AVAILABLE

# Multiple TLD flags
domain-check startup -t com -t org -t io
# Same result as above
```

### Smart TLD Presets

11 built-in presets for common domain search scenarios. Use `--list-presets` to see all available presets from the CLI. All presets work with bootstrap (enabled by default), which resolves TLDs not in the hardcoded registry via IANA.

| Preset | Count | TLDs |
|--------|-------|------|
| `startup` | 8 | com, org, io, ai, tech, app, dev, xyz |
| `popular` | 11 | com, net, org, io, ai, app, dev, tech, me, co, xyz |
| `classic` | 5 | com, net, org, info, biz |
| `enterprise` | 6 | com, org, net, info, biz, us |
| `tech` | 12 | io, ai, app, dev, tech, cloud, software, digital, codes, systems, network, solutions |
| `creative` | 10 | design, art, studio, media, photography, film, music, gallery, graphics, ink |
| `ecommerce` | 8 | shop, store, market, sale, deals, shopping, buy, bargains |
| `finance` | 9 | finance, capital, fund, money, investments, insurance, tax, exchange, trading |
| `web` | 9 | web, site, website, online, blog, page, wiki, host, email |
| `trendy` | 13 | xyz, online, site, top, icu, fun, space, click, website, life, world, live, today |
| `country` | 9 | us, uk, de, fr, ca, au, br, in, nl |

```bash
# Tech startup
domain-check myapp --preset startup
# Checks: .com, .org, .io, .ai, .tech, .app, .dev, .xyz

# All-rounder
domain-check mybrand --preset popular
# Checks: .com, .net, .org, .io, .ai, .app, .dev, .tech, .me, .co, .xyz

# Corporate
domain-check mybrand --preset enterprise
# Checks: .com, .org, .net, .info, .biz, .us

# Online store
domain-check myshop --preset ecommerce
# Checks: .shop, .store, .market, .sale, .deals, .shopping, .buy, .bargains

# Creative agency
domain-check mystudio --preset creative
# Checks: .design, .art, .studio, .media, .photography, .film, .music, .gallery, .graphics, .ink

# Country codes
domain-check mysite --preset country
# Checks: .us, .uk, .de, .fr, .ca, .au, .br, .in, .nl
```

### Universal TLD Checking

With bootstrap enabled (the default), `--all` checks against 1,200+ TLDs — virtually every TLD on the internet.

```bash
# Check against all known TLDs (1,200+ with bootstrap)
domain-check myapp --all
# Fetches the IANA RDAP registry, then checks across all discovered TLDs

# With streaming for real-time results
domain-check myapp --all --streaming
# Shows results as they complete

# Restrict to the 32 hardcoded TLDs (no network bootstrap fetch)
domain-check myapp --all --no-bootstrap
# Faster, offline-capable, but limited to hardcoded TLDs

# Check a TLD not in the hardcoded list (bootstrap handles it automatically)
domain-check example.museum
# Bootstrap discovers the RDAP endpoint for .museum via IANA
```

---

## 🎯 Custom Presets

### Defining Custom Presets

Create reusable TLD combinations in your configuration file:

```toml
# domain-check.toml
[custom_presets]
my_startup = ["com", "io", "ai", "dev", "app", "tech"]
my_crypto = ["com", "org", "crypto", "blockchain", "web3"]
my_enterprise = ["com", "org", "net", "info", "biz"]
my_international = ["com", "org", "uk", "de", "fr", "jp"]
```

### Using Custom Presets

```bash
# Use custom preset via CLI
domain-check mystartup --preset my_crypto

# Use custom preset via environment variable
DC_PRESET=my_startup domain-check mystartup

# Custom presets override built-in presets with same name
domain-check mystartup --preset startup  # Uses your custom 'startup' if defined
```

### Preset Precedence

1. **Custom presets** (from config files) override built-in presets
2. **Built-in presets** used if no custom preset with same name exists
3. **Available built-in presets**: startup, popular, classic, enterprise, tech, creative, ecommerce, finance, web, trendy, country

---

## Output Formats

### Default Output
```bash
domain-check example.com google.com
# example.com TAKEN
# google.com TAKEN
#
# 2 domains in 0.1s  |  0 available  |  2 taken  |  0 unknown
```

Default mode includes colored status words (green AVAILABLE, red TAKEN, yellow UNKNOWN),
a progress counter for multi-domain checks, and a colored summary bar.

### Pretty Output
```bash
domain-check rustcloud --preset startup --pretty --batch
# domain-check v0.9.0 — Checking 8 domains
# Preset: startup | Concurrency: 20
#
# ── Available (3) ──────────────────────────────
#   rustcloud.org
#   rustcloud.ai
#   rustcloud.app
#
# ── Taken (5) ──────────────────────────────────
#   rustcloud.com
#   rustcloud.io
#   rustcloud.tech
#   rustcloud.dev
#   rustcloud.xyz
#
# 8 domains in 0.8s  |  3 available  |  5 taken  |  0 unknown
```

Pretty mode groups results by status (Available/Taken/Unknown), adds a styled header,
column-aligned domain names, and section separators. Empty sections are omitted.

### Detailed Information
```bash
domain-check google.com --info
# google.com TAKEN (Registrar: MarkMonitor Inc., Created: 1997-09-15, Expires: 2028-09-14)
```

### JSON Output
```bash
domain-check example.com --json
```
```json
[
  {
    "domain": "example.com",
    "available": false,
    "method_used": "rdap",
    "check_duration": { "secs": 0, "nanos": 234567890 }
  }
]
```

### CSV Output
```bash
domain-check example.com startup.org --csv
```
```csv
domain,available,registrar,created,expires,method
example.com,false,Example Inc.,1995-08-14,2025-08-13,rdap
startup.org,true,-,-,-,rdap
```

---

## File Processing

### Basic File Input
Create `domains.txt`:
```text
# Startup ideas
unicorn-startup
my-awesome-app
cool-domain

# Existing domains  
google.com
github.com
```

```bash
# Process file with TLD expansion
domain-check --file domains.txt -t com,org
# unicorn-startup.com AVAILABLE
# unicorn-startup.org AVAILABLE  
# my-awesome-app.com AVAILABLE
# my-awesome-app.org AVAILABLE
# cool-domain.com TAKEN
# cool-domain.org AVAILABLE
# google.com TAKEN
# github.com TAKEN
```

### File with Mixed Content
```text
# domains-mixed.txt
example           # Will expand with -t flag
test.com         # FQDN, no expansion
startup          # Will expand
api.example.org  # FQDN, no expansion
```

```bash
domain-check --file domains-mixed.txt -t com,io
# example.com TAKEN
# example.io AVAILABLE
# test.com TAKEN
# startup.com TAKEN  
# startup.io AVAILABLE
# api.example.org TAKEN
```

### Large File Processing
```bash
# High concurrency for large files
domain-check --file large-list.txt -t com --concurrency 50 --streaming

# CSV output for spreadsheet analysis
domain-check --file domains.txt --preset startup --csv > results.csv

# Force processing beyond safety limits
domain-check --file massive-list.txt --force --streaming
```

---

## Performance & Concurrency

### Concurrency Settings
```bash
# Default concurrency (20)
domain-check --file domains.txt -t com,org

# High concurrency for faster processing
domain-check --file domains.txt -t com,org --concurrency 50

# Maximum concurrency (100)
domain-check --file domains.txt --all --concurrency 100
```

### Processing Modes

#### Streaming Mode (Real-time Results)
```bash
domain-check --file domains.txt --all --streaming
# [1/32] example.com TAKEN
# [2/32] example.org AVAILABLE
# [3/32] example.io TAKEN
# ... (results appear as they complete with progress counter)
```

#### Batch Mode (Collected Results)
```bash
domain-check --file domains.txt --preset startup --batch
# (spinner shown while checking...)
# example.com TAKEN
# example.org AVAILABLE
# example.io TAKEN
# ...
# 8 domains in 0.8s  |  3 available  |  5 taken  |  0 unknown
```

Both modes include colored output and a summary bar. Batch mode shows a loading
spinner while waiting for results. In pretty mode, batch results are grouped by status.

---

## Domain Generation

Generate domain name candidates using patterns, prefixes, and suffixes. Generation produces base names that are then expanded with TLDs just like regular domain inputs.

### Pattern Expansion

Use `--pattern` to generate names from wildcard patterns:

| Token | Expands to | Count |
|-------|-----------|-------|
| `\d` | 0-9 | 10 |
| `\w` | a-z + hyphen (not at start/end) | 27 |
| `?` | a-z + 0-9 + hyphen (not at start/end) | 37 |
| Literal | Itself | 1 |

```bash
# Generate test0.com through test9.com
domain-check --pattern "test\d" -t com --dry-run
# test0.com
# test1.com
# ...
# test9.com
# 10 domains would be checked

# Two-digit patterns: app00 through app99
domain-check --pattern "app\d\d" -t com --dry-run
# 100 domains would be checked

# Letter patterns: a-z prefix
domain-check --pattern "\wapp" -t com --dry-run
# 27 domains would be checked (aapp, bapp, ..., zapp, -app filtered)

# Mixed: alphanumeric wildcard
domain-check --pattern "go?" -t com --dry-run
# 37 domains would be checked
```

### Prefix & Suffix Permutations

Use `--prefix` and `--suffix` to generate name combinations:

```bash
# Prefixes only
domain-check app --prefix get,my,try -t com --dry-run
# getapp.com
# myapp.com
# tryapp.com
# app.com          (bare name included by default)
# 4 domains would be checked

# Suffixes only
domain-check cloud --suffix hub,ly,io -t com --dry-run
# cloudhub.com
# cloudly.com
# cloudio.com
# cloud.com
# 4 domains would be checked

# Both prefixes and suffixes
domain-check app --prefix get,my --suffix hub,ly -t com --dry-run
# getapphub.com
# getapply.com
# getapp.com
# myapphub.com
# myapply.com
# myapp.com
# apphub.com
# apply.com
# app.com
# 9 domains would be checked
```

### Combining Patterns with Affixes

Patterns and affixes compose naturally:

```bash
# Generate test0-test9, then prepend "get" and "my"
domain-check --pattern "test\d" --prefix get,my -t com --dry-run
# 30 domains would be checked (10 names × 3 variants × 1 TLD)

# Pattern + suffix + multiple TLDs
domain-check --pattern "app\d" --suffix hub -t com,org --dry-run
# 40 domains would be checked (10 names × 2 variants × 2 TLDs)
```

### Dry Run

Preview what would be checked without making any network requests:

```bash
# Plain text list
domain-check --pattern "test\d" -t com --dry-run

# JSON output for piping
domain-check --pattern "test\d" -t com --dry-run --json

# Count domains in a complex generation
domain-check --pattern "app\d\d" --prefix get,my --preset startup --dry-run 2>&1 | tail -1
# 2400 domains would be checked
```

### Interactive Confirmation

For large runs (>5,000 domains), domain-check asks for confirmation in interactive terminals:

```bash
# This will prompt before checking
domain-check --pattern "test\d\d" --preset startup
# Will check 800 domains (~40s at concurrency 20). Proceed? [Y/n]

# Skip the prompt for automation
domain-check --pattern "test\d\d" --preset startup --yes

# Also skipped with --force
domain-check --pattern "test\d\d" --preset startup --force

# Non-TTY (piped) never prompts — safe for agents and scripts
domain-check --pattern "test\d" -t com --json | jq '.'
```

### Generation CLI Flags

| Flag | Description | Example |
|------|-------------|---------|
| `--pattern <PAT>` | Pattern for name generation | `--pattern "test\d"` |
| `--prefix <LIST>` | Comma-separated prefixes | `--prefix get,my,try` |
| `--suffix <LIST>` | Comma-separated suffixes | `--suffix hub,ly,app` |
| `--dry-run` | Preview domains without checking | `--dry-run` |
| `-y, --yes` | Skip confirmation prompts | `--yes` |

### Config File & Env Var Defaults

Prefixes and suffixes can be set as persistent defaults:

```toml
# domain-check.toml
[generation]
prefixes = ["get", "my"]
suffixes = ["hub", "ly"]
```

```bash
# Environment variables
DC_PREFIX=get,my domain-check app -t com --dry-run
DC_SUFFIX=hub,ly domain-check app -t com --dry-run
```

CLI flags override env vars, which override config file values.

**Note:** `--pattern` is intentionally excluded from config/env — patterns are per-invocation exploratory inputs, not persistent defaults.

---

## Advanced Features

### Bootstrap & Protocol Discovery
```bash
# Bootstrap is enabled by default — any TLD works out of the box
domain-check example.restaurant --debug
# Bootstrap: loaded 1,180 TLDs from IANA RDAP registry
# Found endpoint: https://rdap.donuts.co/domain/
# example.restaurant TAKEN

# For TLDs without RDAP, WHOIS server is discovered automatically
domain-check example.es --debug
# No RDAP endpoint for .es
# WHOIS: discovered whois.nic.es via IANA referral
# example.es TAKEN

# Disable bootstrap for offline/faster operation (32 hardcoded TLDs only)
domain-check example.com --no-bootstrap
```

### Protocol Control
```bash
# Disable WHOIS fallback (RDAP only)
domain-check example.com --no-whois

# Enable debug output
domain-check example.com --debug
# Shows detailed protocol information, discovery steps, and timing
```

### Complex Queries
```bash
# Multiple domains with presets and output formatting
domain-check startup unicorn coolapp --preset startup --json --info > results.json

# File processing with detailed info and CSV output
domain-check --file brand-check.txt --preset enterprise --info --csv > brand-audit.csv
```

---

## Tips & Tricks

### Domain Name Intelligence
```bash
# Base names automatically expand
domain-check startup -t com,org     # → startup.com, startup.org

# FQDNs don't expand  
domain-check startup.com -t org     # → startup.com only

# Mix base names and FQDNs
domain-check startup test.com -t io # → startup.io, test.com
```

### Pipeline Integration
```bash
# Find available .com domains
domain-check --file list.txt -t com --json | jq -r '.[] | select(.available==true) | .domain'

# Count available domains by TLD
domain-check --file list.txt --preset startup --csv | grep ",true," | cut -d',' -f1 | sort

# Filter for specific registrars  
domain-check --file domains.txt --info --json | jq '.[] | select(.info.registrar | contains("GoDaddy"))'
```

### Domain Generation Workflows
```bash
# Explore 3-letter .com domains
domain-check --pattern "\w\w\w" -t com --dry-run | wc -l
# 19683 names (27^3) — preview before committing

# Find available "get*" startup domains
domain-check --pattern "get\w\w\w" --preset startup --batch --json > get-domains.json

# AI agent integration: generate + check + filter (non-interactive)
domain-check --pattern "app\d" --prefix get,my -t com --yes --json | \
  jq -r '.[] | select(.available==true) | .domain'

# Brand variations with prefixes/suffixes
domain-check mybrand --prefix get,try,use --suffix app,hub,io -t com --dry-run

# Config-driven generation (uses domain-check.toml prefixes/suffixes)
domain-check myapp -t com,io
```

### Performance Optimization
```bash
# For 1-50 domains: use defaults
domain-check --file small.txt -t com,org

# For 50-200 domains: increase concurrency
domain-check --file medium.txt --preset startup --concurrency 25

# For 200+ domains: high concurrency + streaming
domain-check --file large.txt --all --concurrency 50 --streaming
```

### File Format Best Practices
```text
# domains.txt - Well-structured input file

# === SECTION: Startup Ideas ===
ai-startup
ml-platform  
data-wizard

# === SECTION: Brand Variations ===
mybrand
my-brand
mybrand-app
mybrand-official

# === SECTION: Existing Domains ===
google.com        # Reference check
github.com        # Reference check
```

### Error Handling & Debugging
```bash
# Verbose output for troubleshooting
domain-check problematic-domain.com --verbose --debug

# Check specific protocols
domain-check example.com --no-whois     # RDAP only
domain-check example.com --debug        # Show protocol discovery details
```

### Automation Scripts
```bash
#!/bin/bash
# Daily domain monitoring script

echo "Checking startup domains..."
domain-check --file startup-ideas.txt --preset startup --csv > "check-$(date +%Y%m%d).csv"

echo "Checking brand protection..."  
domain-check --file brand-variations.txt --preset enterprise --json > "brand-$(date +%Y%m%d).json"

echo "Reports generated!"
```

### Configuration-Driven Workflows

```bash
# Team Configuration
# Create shared team-config.toml in repository
domain-check --config team-config.toml --file project-domains.txt

# Personal Defaults
# Set up ~/.domain-check.toml once
cat > ~/.domain-check.toml << 'EOF'
[defaults]
concurrency = 30
pretty = true
preset = "startup"

[generation]
prefixes = ["get", "my", "try"]
suffixes = ["hub", "app"]
EOF

# Now all commands use your preferences
domain-check mystartup  # Automatic startup preset, pretty output, 30 concurrency
# Prefixes/suffixes auto-applied from config:
domain-check myapp -t com  # → getmyapp, mymyapp, trymyapp, myapphub, myappapp, myapp

# Environment-Specific Settings
# Development
DC_CONCURRENCY=10 domain-check mystartup

# Production CI
DC_CONCURRENCY=50 DC_TIMEOUT=30s domain-check --file critical-domains.txt
```

### Configuration Debugging

```bash
# See which config files are being used
domain-check mystartup --verbose

# Test specific configuration
domain-check --config test-config.toml mystartup --verbose

# Override everything with CLI
domain-check mystartup --concurrency 1 --preset enterprise --batch
```

---

## Example Workflows

### Domain Investor Workflow
```bash
# 1. Generate ideas list
echo -e "ai-startup\nml-platform\ndata-wizard" > ideas.txt

# 2. Check premium TLDs with detailed info
domain-check --file ideas.txt -t com,io,ai --info --csv > portfolio-analysis.csv

# 3. Find available premium domains
domain-check --file ideas.txt -t com,io,ai --json | jq '.[] | select(.available==true)'
```

### Startup Name Research
```bash
# 1. Check core name across startup TLDs
domain-check mycoolstartup --preset startup --pretty

# 2. Check variations
echo -e "mycoolstartup\nmy-cool-startup\nmycoolstartup-app" > variations.txt
domain-check --file variations.txt --preset startup --streaming

# 3. Export for team review
domain-check --file variations.txt --preset startup --csv > name-options.csv
```

### Brand Protection Monitoring
```bash
# 1. Create monitoring list
echo -e "mybrand\nmy-brand\nmybrand-app\nmybrand-official" > brand-check.txt

# 2. Check across business TLDs
domain-check --file brand-check.txt --preset enterprise --info --json > brand-status.json

# 3. Set up alerts for changes
# (integrate with your monitoring system)
```

---

*For library integration examples, see the [Library Documentation](https://docs.rs/domain-check-lib).*
