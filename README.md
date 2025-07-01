# domain-check

**Fast, powerful CLI tool for checking domain availability using RDAP and WHOIS protocols.**

[![Homebrew](https://img.shields.io/badge/Homebrew-available-brightgreen)](https://github.com/saidutt46/homebrew-domain-check)
[![CLI Crate](https://img.shields.io/crates/v/domain-check.svg?label=CLI)](https://crates.io/crates/domain-check)
[![Library Crate](https://img.shields.io/crates/v/domain-check-lib.svg?label=Library)](https://crates.io/crates/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check.svg)](https://crates.io/crates/domain-check)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

<p align="center">
  <img src="./assets/demov0.5.1.svg" alt="Basic Usage" width="700"/>
</p>

---

## Why domain-check?

Tired of switching between browser tabs and WHOIS websites to check if domains are available? **domain-check** brings fast, accurate domain availability checking directly to your terminal. Built in Rust for speed, with smart presets for common scenarios, and bulk processing for when you need to check hundreds of domains at once.

Perfect for developers, domain investors, startups, and anyone who works with domains regularly.

---

## 📦 Installation

### Homebrew (macOS)
```bash
brew tap saidutt46/domain-check
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
# 🔴 example.com is TAKEN
```

### Check multiple TLD variations
```bash
domain-check mystartup -t com,io,ai,dev
# 🔍 Checking 4 domains...
# 🔴 mystartup.com is TAKEN
# 🟢 mystartup.io is AVAILABLE
# 🟢 mystartup.ai is AVAILABLE  
# 🟢 mystartup.dev is AVAILABLE
```

### Check ALL TLDs at once
```bash
# Check against ALL 35+ known TLDs in seconds
domain-check myapp --all
# 🔍 Checking 35+ domains across all TLDs...
# 🟢 myapp.com is AVAILABLE
# 🔴 myapp.io is TAKEN  
# 🟢 myapp.ai is AVAILABLE
# 🟢 myapp.dev is AVAILABLE
# ... (38 more results in ~2 seconds)
```

### Use smart presets
```bash
# Check against startup-focused TLDs (8 TLDs)
domain-check myapp --preset startup

# Enterprise-focused TLDs (6 TLDs)
domain-check mybrand --preset enterprise
```

### Bulk check from file
```bash
echo -e "myapp\nmystartup\ncoolproject" > domains.txt
domain-check --file domains.txt -t com,org --json > results.json
```

---

## 🎯 Choose Your Path

**Just need CLI?** You're all set! Check out our [CLI Examples](./docs/CLI.md) for advanced usage patterns.

**Building a Rust app?** Use our library: 
```toml
[dependencies]
domain-check-lib = "0.5.1"
```
See the [Library Documentation](https://docs.rs/domain-check-lib) for integration examples.

**Need bulk domain processing?** See [Advanced Examples](./docs/EXAMPLES.md) for enterprise workflows.

---

## ✨ Key Features

🌐 **Universal Coverage** - Check against ALL 35+ TLDs with `--all` or use smart presets  
⚡ **Lightning Fast** - Concurrent processing up to 100 domains simultaneously  
📊 **Rich Output Options** - Beautiful terminal display, JSON/CSV for automation, detailed info mode  
📁 **Bulk Processing** - Process thousands of domains from files with real-time streaming results

---

## 🔗 Resources

- **CLI Documentation**: [Command Reference & Examples](./docs/CLI.md) *(coming soon)*
- **Library Documentation**: [docs.rs/domain-check-lib](https://docs.rs/domain-check-lib)
- **Advanced Examples**: [Enterprise Workflows](./docs/EXAMPLES.md) *(coming soon)*
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