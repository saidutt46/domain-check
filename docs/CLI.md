# CLI Reference & Examples

Complete guide to using the `domain-check` command-line tool.

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
# .domain-check.toml
[defaults]
concurrency = 25
preset = "startup"
pretty = true
timeout = "8s"
bootstrap = true

[custom_presets]
my_startup = ["com", "io", "ai", "dev", "app"]
my_enterprise = ["com", "org", "net", "biz", "info"]

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
3. **Local config** (./.domain-check.toml)
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
| `DC_BOOTSTRAP` | `--bootstrap` | `DC_BOOTSTRAP=true` | Enable IANA bootstrap |
| `DC_WHOIS_FALLBACK` | `--no-whois` | `DC_WHOIS_FALLBACK=false` | WHOIS fallback |
| `DC_DETAILED_INFO` | `--info` | `DC_DETAILED_INFO=true` | Detailed domain info |
| `DC_JSON` | `--json` | `DC_JSON=true` | JSON output format |
| `DC_CSV` | `--csv` | `DC_CSV=true` | CSV output format |
| `DC_FILE` | `--file` | `DC_FILE=domains.txt` | Default domains file |
| `DC_CONFIG` | `--config` | `DC_CONFIG=my-config.toml` | Default config file |

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
| `--all` | Check against all 32 known TLDs | `domain-check myapp --all` |
| `--preset <NAME>` | Use TLD preset or custom preset | `domain-check myapp --preset startup` |
| `-f, --file <FILE>` | Read domains from file | `domain-check --file domains.txt` |
| `--config <FILE>` | Use specific config file | `domain-check --config my-config.toml` |
| `-h, --help` | Show help information | `domain-check --help` |
| `-V, --version` | Show version | `domain-check --version` |

### TLD Selection

| Flag | Description | Example |
|------|-------------|---------|
| `-t, --tld <TLD>` | Specify TLDs for base names | `domain-check startup -t com,org,io` |
| `--all` | Check against all 32 known TLDs | `domain-check myapp --all` |
| `--preset <NAME>` | Use TLD preset (startup/enterprise/country) | `domain-check myapp --preset startup` |

### Input Sources

| Flag | Description | Example |
|------|-------------|---------|
| `-f, --file <FILE>` | Read domains from file | `domain-check --file domains.txt` |

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
| `-b, --bootstrap` | Use IANA bootstrap for unknown TLDs | `domain-check example.rare --bootstrap` |
| `--no-whois` | Disable WHOIS fallback | `domain-check example.com --no-whois` |

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

#### Startup Preset (8 TLDs)
```bash
domain-check myapp --preset startup
# Checks: .com, .org, .io, .ai, .tech, .app, .dev, .xyz
```

#### Enterprise Preset (6 TLDs)
```bash
domain-check mybrand --preset enterprise  
# Checks: .com, .org, .net, .info, .biz, .us
```

#### Country Preset (9 TLDs)
```bash
domain-check mysite --preset country
# Checks: .us, .uk, .de, .fr, .ca, .au, .br, .in, .nl
```

### Universal TLD Checking
```bash
# Check against all 32 known TLDs
domain-check myapp --all
# Checks all TLDs with RDAP endpoints

# With streaming for real-time results
domain-check myapp --all --streaming
# Shows results as they complete
```

---

## 🎯 Custom Presets

### Defining Custom Presets

Create reusable TLD combinations in your configuration file:

```toml
# .domain-check.toml
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
3. **Available built-in presets**: startup, enterprise, country

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
# domain-check v0.7.0 — Checking 8 domains
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

## Advanced Features

### Bootstrap Registry Discovery
```bash
# For unknown or new TLDs
domain-check example.restaurant --bootstrap --debug
# Trying IANA bootstrap for .restaurant...
# Found endpoint: https://rdap.donuts.co/domain/
# example.restaurant TAKEN
```

### Protocol Control
```bash
# Disable WHOIS fallback (RDAP only)
domain-check example.com --no-whois

# Enable debug output
domain-check example.com --debug
# Shows detailed protocol information and timing
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
domain-check example.com --bootstrap    # Enable discovery
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
EOF

# Now all commands use your preferences
domain-check mystartup  # Automatic startup preset, pretty output, 30 concurrency

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