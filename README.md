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

---

## Why domain-check?

Tired of switching between browser tabs and WHOIS websites to check if domains are available? **domain-check** brings fast, accurate domain availability checking directly to your terminal. Built in Rust for speed, with **configuration files**, **environment variables**, **custom presets**, and bulk processing for when you need to check hundreds of domains at once.

Perfect for developers, domain investors, startups, and anyone who works with domains regularly.

---

## 📦 Installation

### Homebrew (macOS)
```bash
brew install domain-check
```

### Cargo (All Platforms)
```bash
cargo install domain-check
```

### Download Binaries
Pre-built binaries available for macOS, Linux, and Windows: [GitHub Releases](https://github.com/saidutt46/domain-check/releases)

---

## 🚀 Quick Start

### Check a single domain
```bash
domain-check example.com
# example.com TAKEN
```

### Check multiple TLD variations
```bash
domain-check mystartup -t com,org,net,dev --batch
# mystartup.com TAKEN
# mystartup.org TAKEN
# mystartup.net TAKEN
# mystartup.dev AVAILABLE
#
# 4 domains in 0.1s  |  1 available  |  3 taken  |  0 unknown
```

### Check ALL TLDs at once
```bash
# Check against all 32 known TLDs in seconds
domain-check myapp --all --batch
```

### Use smart presets
```bash
# Check against startup-focused TLDs (8 TLDs)
domain-check myapp --preset startup

# Enterprise-focused TLDs (6 TLDs)
domain-check mybrand --preset enterprise
```

### Pretty mode — grouped results
```bash
domain-check rustcloud --preset startup --pretty --batch
# domain-check v0.6.1 — Checking 8 domains
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

### Bulk check from file
```bash
echo -e "myapp\nmystartup\ncoolproject" > domains.txt
domain-check --file domains.txt -t com,org --json > results.json
```

---

## ⚙️ Configuration & Customization

### Configuration Files

Create persistent settings with configuration files:

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
my_crypto = ["com", "org", "crypto", "blockchain"]

[output]
default_format = "pretty"
csv_headers = true
```

**Configuration file locations (checked in order):**
- `./domain-check.toml` (project-specific)
- `~/.domain-check.toml` (user global)
- `~/.config/domain-check/config.toml` (XDG standard)

### Environment Variables

Perfect for CI/CD and automation:

```bash
# Basic settings
export DC_CONCURRENCY=50
export DC_PRESET=startup
export DC_PRETTY=true
export DC_BOOTSTRAP=true

# File locations
export DC_CONFIG=/path/to/config.toml
export DC_FILE=/path/to/domains.txt

# Use in commands
DC_TIMEOUT=30s domain-check mystartup
```

### Custom Presets

Define your own TLD combinations:

```bash
# Use custom preset from config file
domain-check mystartup --preset my_crypto

# Via environment variable  
DC_PRESET=my_startup domain-check mystartup
```

---

## 📖 Command Reference

### Usage
```
domain-check [OPTIONS] [DOMAINS]...
```

### Arguments
- `<DOMAINS>...` - Domain names to check (supports both base names and FQDNs)

### Core Options
| Option | Description | Example |
|--------|-------------|---------|
| `-t, --tld <TLD>` | Specify TLDs for base names | `-t com,org,io` |
| `--all` | Check against all 32 known TLDs | `--all` |
| `--preset <NAME>` | Use TLD preset (startup/enterprise/country) | `--preset startup` |
| `-f, --file <FILE>` | Read domains from file | `-f domains.txt` |
| `--config <FILE>` | Use specific config file | `--config my-config.toml` |

### Performance Options
| Option | Description | Default |
|--------|-------------|---------|
| `-c, --concurrency <N>` | Max concurrent checks (1-100) | `20` |
| `--force` | Override safety limits | |
| `--streaming` | Show results as they complete | |
| `--batch` | Collect all results before showing | |

### Output Options
| Option | Description | Example |
|--------|-------------|---------|
| `-p, --pretty` | Grouped, structured output with section headers | `--pretty` |
| `-j, --json` | Output in JSON format | `--json` |
| `--csv` | Output in CSV format | `--csv` |
| `-i, --info` | Show detailed domain information | `--info` |

### Protocol Options
| Option | Description | Default |
|--------|-------------|---------|
| `-b, --bootstrap` | Use IANA bootstrap for unknown TLDs | `false` |
| `--no-whois` | Disable WHOIS fallback | `false` |

### Debugging
| Option | Description |
|--------|-------------|
| `-d, --debug` | Show detailed debug information |
| `-v, --verbose` | Enable verbose logging |
| `-h, --help` | Show help information |
| `-V, --version` | Show version |

### Environment Variables
| Variable | Description | Example |
|----------|-------------|---------|
| `DC_CONCURRENCY` | Default concurrency | `DC_CONCURRENCY=50` |
| `DC_PRESET` | Default preset | `DC_PRESET=startup` |
| `DC_TLD` | Default TLD list | `DC_TLD=com,io,dev` |
| `DC_PRETTY` | Enable pretty output | `DC_PRETTY=true` |
| `DC_TIMEOUT` | Request timeout | `DC_TIMEOUT=10s` |
| `DC_BOOTSTRAP` | Enable bootstrap | `DC_BOOTSTRAP=true` |
| `DC_CONFIG` | Config file path | `DC_CONFIG=my-config.toml` |
| `DC_FILE` | Domains file path | `DC_FILE=domains.txt` |

---

## 🎯 Choose Your Path

**Just need CLI?** You're all set! Check out our [CLI Examples](./docs/CLI.md) for advanced usage patterns.

**Building a Rust app?** Use our library: 
```toml
[dependencies]
domain-check-lib = "0.7.0"
```
See the [Library Documentation](https://docs.rs/domain-check-lib) for integration examples.

**Need bulk domain processing?** See [Advanced Examples](./docs/EXAMPLES.md) for enterprise workflows.

---

## ✨ Key Features

🌐 **Universal Coverage** - Check against all 32 TLDs with `--all` or use smart presets
⚡ **Lightning Fast** - Concurrent processing up to 100 domains simultaneously
🎨 **Beautiful Output** - Colored results, progress counters, loading spinner, grouped pretty mode
📊 **Multiple Formats** - Pretty terminal display, JSON, CSV for automation, detailed info mode
📁 **Bulk Processing** - Process thousands of domains from files with real-time streaming results
⚙️ **Configuration Files** - Persistent settings with TOML configuration files
🔧 **Environment Variables** - Full DC_* environment variable support for automation
🎯 **Custom Presets** - Define your own TLD combinations for repeated use

---

## 📋 Examples

### Basic Usage
```bash
# Single domain
domain-check example.com

# Multiple domains with pretty output
domain-check example.com google.com --pretty

# Check startup-focused TLDs
domain-check mystartup --preset startup

# Check all available TLDs
domain-check myapp --all --streaming
```

### Configuration-Driven Workflow
```bash
# One-time setup
cat > .domain-check.toml << 'EOF'
[defaults]
concurrency = 25
preset = "startup"
pretty = true

[custom_presets]
my_stack = ["com", "io", "dev", "app"]
EOF

# Now simple commands use your preferences
domain-check mystartup        # Uses startup preset, pretty output, 25 concurrency
domain-check --preset my_stack myproject  # Uses custom preset
```

### Automation & CI/CD
```bash
# Environment-driven configuration
DC_CONCURRENCY=50 DC_PRESET=enterprise domain-check --file domains.txt --json

# Docker usage
docker run -e DC_PRESET=startup domain-check:latest myapp

# GitHub Actions
- name: Check domains
  run: |
    export DC_CONCURRENCY=30
    domain-check --file required-domains.txt --json
```

### Advanced Workflows
```bash
# Domain research pipeline
domain-check --file ideas.txt --preset startup --csv > research.csv

# Brand protection monitoring  
domain-check --file brand-variations.txt --all --json > monitoring.json

# Performance testing with high concurrency
domain-check --file large-list.txt --concurrency 75 --streaming
```

---

## 🔗 Resources

- **CLI Documentation**: [Command Reference & Examples](./docs/CLI.md)
- **Library Documentation**: [docs.rs/domain-check-lib](https://docs.rs/domain-check-lib)
- **Advanced Examples**: [Enterprise Workflows](./docs/EXAMPLES.md)
- **Changelog**: [CHANGELOG.md](./CHANGELOG.md)
- **Contributing**: [CONTRIBUTING.md](./CONTRIBUTING.md)

### Crates
- **CLI Tool**: [crates.io/crates/domain-check](https://crates.io/crates/domain-check)
- **Library**: [crates.io/crates/domain-check-lib](https://crates.io/crates/domain-check-lib)

---

## 📝 License

Licensed under the Apache License, Version 2.0 - see the [LICENSE](LICENSE) file for details.

---

*Built with ❤️ in Rust*
