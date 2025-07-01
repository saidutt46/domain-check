# CLI Reference & Examples

Complete guide to using the `domain-check` command-line tool.

## Table of Contents

- [Basic Usage](#basic-usage)
- [Command Reference](#command-reference)
- [TLD Options](#tld-options)
- [Output Formats](#output-formats)
- [File Processing](#file-processing)
- [Performance & Concurrency](#performance--concurrency)
- [Advanced Features](#advanced-features)
- [Tips & Tricks](#tips--tricks)

---

## Basic Usage

### Single Domain Check
```bash
# Basic check (plain output)
domain-check example.com
# example.com TAKEN

# Pretty output with colors and emojis
domain-check example.com --pretty
# 🔴 example.com is TAKEN
```

### Multiple Domain Arguments
```bash
# Check multiple domains at once
domain-check example.com google.com startup.org
# example.com TAKEN
# google.com TAKEN  
# startup.org AVAILABLE
```

---

## Command Reference

### Core Options

| Flag | Description | Example |
|------|-------------|---------|
| `<DOMAINS>...` | Domain names to check | `domain-check example.com google.com` |
| `-h, --help` | Show help information | `domain-check --help` |
| `-V, --version` | Show version | `domain-check --version` |

### TLD Selection

| Flag | Description | Example |
|------|-------------|---------|
| `-t, --tld <TLD>` | Specify TLDs for base names | `domain-check startup -t com,org,io` |
| `--all` | Check against all 42+ known TLDs | `domain-check myapp --all` |
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
| `-p, --pretty` | Colorful output with emojis | `domain-check example.com --pretty` |
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
# Checks: .us, .uk, .de, .fr, .ca, .au, .jp, .br, .in
```

### Universal TLD Checking
```bash
# Check against ALL known TLDs
domain-check myapp --all
# Checks 42+ TLDs automatically

# With streaming for real-time results
domain-check myapp --all --streaming
# Shows results as they complete
```

---

## Output Formats

### Default Output
```bash
domain-check example.com google.com
# example.com TAKEN
# google.com TAKEN
```

### Pretty Output
```bash
domain-check example.com google.com --pretty
# 🔴 example.com is TAKEN
# 🔴 google.com is TAKEN
```

### Detailed Information
```bash
domain-check google.com --info --pretty
# 🔴 google.com is TAKEN (Registrar: MarkMonitor Inc., Created: 1997-09-15, Expires: 2028-09-14)
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
# Default concurrency (10)
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
# 🔍 Checking 42 domains with concurrency: 10
# 🟢 example.com is AVAILABLE
# 🔴 test.org is TAKEN
# 🟢 startup.io is AVAILABLE
# ... (results appear as they complete)
```

#### Batch Mode (Collected Results)
```bash
domain-check --file domains.txt --preset startup --batch
# 🔍 Checking 8 domains...
# (waits for all results)
# 🟢 example.com is AVAILABLE
# 🔴 example.org is TAKEN
# 🟢 example.io is AVAILABLE
# ... (all results at once)
```

---

## Advanced Features

### Bootstrap Registry Discovery
```bash
# For unknown or new TLDs
domain-check example.restaurant --bootstrap --debug
# 🔍 No known RDAP endpoint for .restaurant, trying bootstrap registry...
# 🔍 Found endpoint: https://rdap.donuts.co/domain/
# 🔴 example.restaurant is TAKEN
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