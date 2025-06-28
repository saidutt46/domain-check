# Domain Check

**A fast, library-first Rust toolkit for domain availability checking using RDAP and WHOIS protocols**

[![Crates.io](https://img.shields.io/crates/v/domain-check.svg)](https://crates.io/crates/domain-check)
[![Documentation](https://docs.rs/domain-check-lib/badge.svg)](https://docs.rs/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check.svg)](https://crates.io/crates/domain-check)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/saidutt46/domain-check/workflows/CI/badge.svg)](https://github.com/saidutt46/domain-check/actions)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![Security audit](https://github.com/saidutt46/domain-check/workflows/Security%20Audit/badge.svg)](https://github.com/saidutt46/domain-check/actions)

---
## 🎯 **Why Choose Domain Check?**

🦀 **Library + CLI in One** - Use as a Rust library or standalone command-line tool  
⚡ **Lightning Fast** - Concurrent processing with smart rate limiting  
🌐 **Protocol Smart** - RDAP-first with automatic WHOIS fallback  
🎯 **Production Ready** - Used by developers worldwide with enterprise-grade reliability  
📦 **Zero Hassle Setup** - Works out of the box, no configuration needed  
🔒 **Security Focused** - Regular security audits and dependency monitoring  

*Perfect for domain investors, developers, system administrators, and anyone who needs to check domain availability at scale.*
---

## 🚀 Quick Start

### 📦 Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
domain-check-lib = "0.4.0"
tokio = { version = "1", features = ["full"] }
```

**Basic Example:**
```rust
use domain_check_lib::DomainChecker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let result = checker.check_domain("example.com").await?;
    
    println!("Domain: {} - Available: {:?}", result.domain, result.available);
    Ok(())
}
```

### 🖥️ CLI Installation

```bash
cargo install domain-check
```

**Basic Usage:**
```bash
# Single domain
domain-check example.com

# Multiple domains with TLD expansion
domain-check example startup -t com,org,net

# Bulk checking from file
domain-check --file domains.txt -t com,org
```

---

## ✨ Key Features

🦀 **Library-First Architecture** - Clean, async APIs for Rust applications  
⚡ **High Performance** - Concurrent processing with smart rate limiting  
🌐 **Dual Protocol Support** - RDAP-first with automatic WHOIS fallback  
📁 **Flexible Input** - Single domains, bulk files, or programmatic arrays  
🎯 **Smart TLD Expansion** - Automatic expansion of base names across TLDs  
📊 **Multiple Output Formats** - JSON, CSV, or human-readable text  
🔄 **Streaming Results** - Real-time progress for large operations  
🛡️ **Robust Error Handling** - Comprehensive error types and recovery  

---

## 📚 Library Usage Examples

### Single Domain Check

```rust
use domain_check_lib::{DomainChecker, CheckConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    
    match checker.check_domain("google.com").await {
        Ok(result) => {
            println!("Domain: {}", result.domain);
            println!("Available: {:?}", result.available);
            println!("Method: {}", result.method_used);
            
            if let Some(info) = result.info {
                println!("Registrar: {:?}", info.registrar);
                println!("Created: {:?}", info.creation_date);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    
    Ok(())
}
```

### Bulk Domain Checking

```rust
use domain_check_lib::{DomainChecker, CheckConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CheckConfig::default()
        .with_concurrency(20)
        .with_detailed_info(true)
        .with_tlds(vec!["com".to_string(), "org".to_string(), "net".to_string()]);
    
    let checker = DomainChecker::with_config(config);
    let domains = vec!["example".to_string(), "test".to_string(), "startup".to_string()];
    
    let results = checker.check_domains(&domains).await?;
    
    for result in results {
        println!("{}: {:?}", result.domain, result.available);
    }
    
    Ok(())
}
```

### Streaming Results (Real-time)

```rust
use domain_check_lib::DomainChecker;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let domains = vec!["example.com".to_string(), "google.org".to_string()];
    
    let mut stream = checker.check_domains_stream(&domains);
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(domain_result) => {
                println!("✓ {}: {:?}", domain_result.domain, domain_result.available);
            }
            Err(e) => {
                eprintln!("✗ Error: {}", e);
            }
        }
    }
    
    Ok(())
}
```

### File-based Processing

```rust
use domain_check_lib::{DomainChecker, CheckConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CheckConfig::default()
        .with_concurrency(50)
        .with_whois_fallback(true);
    
    let checker = DomainChecker::with_config(config);
    
    // Process domains from file
    let results = checker.check_domains_from_file("domains.txt").await?;
    
    let available: Vec<_> = results.iter()
        .filter(|r| r.available == Some(true))
        .collect();
    
    println!("Found {} available domains out of {}", available.len(), results.len());
    
    Ok(())
}
```

### Custom Configuration

```rust
use domain_check_lib::{DomainChecker, CheckConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CheckConfig::default()
        .with_concurrency(30)                           // Max 30 concurrent checks
        .with_timeout(Duration::from_secs(10))          // 10 second timeout
        .with_whois_fallback(true)                      // Enable WHOIS fallback
        .with_bootstrap(true)                           // Use IANA bootstrap
        .with_detailed_info(true);                      // Extract full domain info
    
    let checker = DomainChecker::with_config(config);
    let result = checker.check_domain("example.com").await?;
    
    println!("Checked via: {}", result.method_used);
    if let Some(duration) = result.check_duration {
        println!("Response time: {}ms", duration.as_millis());
    }
    
    Ok(())
}
```

---

## 🖥️ CLI Usage Examples

### Basic Domain Checking

```bash
# Single domain
domain-check example.com
# Output: example.com TAKEN

# Multiple specific domains
domain-check example.com google.org github.io
```

### Smart TLD Expansion

```bash
# Expand base names across TLDs
domain-check example startup -t com,org,net
# Checks: example.com, example.org, example.net, startup.com, startup.org, startup.net

# Mix FQDNs and base names
domain-check example.com startup -t org,net
# Checks: example.com (no expansion), startup.org, startup.net
```

### Bulk Operations

```bash
# Process domains from file
domain-check --file domains.txt -t com,org

# Override domain limits for large files
domain-check --file large-list.txt --force

# Control concurrency
domain-check --file domains.txt --concurrency 50
```

### Output Formats

```bash
# Pretty colored output (default in terminals)
domain-check example startup -t com,org --pretty

# JSON output for processing
domain-check example startup -t com,org --json

# CSV output for spreadsheets
domain-check example startup -t com,org --csv

# Detailed domain information
domain-check google.com --info
# Output: google.com TAKEN (Registrar: MarkMonitor Inc., Created: 1997-09-15, Expires: 2028-09-14)
```

### Processing Modes

```bash
# Streaming mode (real-time results)
domain-check example startup brand -t com,org,net --streaming

# Batch mode (collect all results first)
domain-check example startup brand -t com,org,net --batch

# Debug mode (detailed protocol information)
domain-check example.com --debug
```

---

## 📦 Installation

### Library (Rust Projects)

Add to your `Cargo.toml`:

```toml
[dependencies]
domain-check-lib = "0.4.0"
tokio = { version = "1", features = ["full"] }

# Optional: for streaming
futures = "0.3"
```

### CLI Tool

#### Via Cargo

```bash
cargo install domain-check
```

#### Pre-built Binaries

Download from [GitHub Releases](https://github.com/saidutt46/domain-check/releases)

#### From Source

```bash
git clone https://github.com/saidutt46/domain-check.git
cd domain-check
cargo build --release
```

---

## 🏗️ Architecture

Domain Check v0.4.0 introduces a **library-first architecture** with two complementary crates:

### Core Library (`domain-check-lib`)

- **Protocol Implementations**: RDAP and WHOIS clients with 30+ TLD mappings
- **Concurrency Engine**: Smart rate limiting and timeout management  
- **Error Handling**: Comprehensive error types with user-friendly messages
- **Zero Dependencies**: Pure async implementation with minimal dependencies

### CLI Application (`domain-check`)

- **User Interface**: Argument parsing and output formatting
- **File Processing**: Bulk domain handling with validation
- **Output Modes**: JSON, CSV, and terminal-optimized text
- **Progress Indicators**: Real-time feedback for long operations

### Protocol Support

- **RDAP (Primary)**: Modern registration data protocol with structured JSON responses
- **WHOIS (Fallback)**: Traditional protocol for maximum TLD coverage
- **Bootstrap Discovery**: IANA registry for unknown TLDs
- **Smart Fallbacks**: Automatic protocol selection and retry logic

---

## 🔧 Configuration

### Library Configuration

```rust
use domain_check_lib::CheckConfig;
use std::time::Duration;

let config = CheckConfig::default()
    .with_concurrency(25)                    // Concurrent requests (1-100)
    .with_timeout(Duration::from_secs(8))    // Per-domain timeout
    .with_whois_fallback(true)               // Enable WHOIS fallback
    .with_bootstrap(false)                   // IANA bootstrap lookup
    .with_detailed_info(true)                // Extract registrar details
    .with_tlds(vec!["com", "org", "net"]);   // Default TLDs for expansion
```

### CLI Configuration

```bash
# Configuration via arguments
domain-check example \
  --tld com,org,net \
  --concurrency 30 \
  --info \
  --bootstrap \
  --pretty

# Environment variables
export DOMAIN_CHECK_DEBUG_RDAP=1    # Enable RDAP debugging
export DOMAIN_CHECK_TIMEOUT=10      # Default timeout in seconds
```

---

## 📄 File Format

Domain files support comments and flexible formatting:

```text
# Startup ideas to check
example
startup-idea
my-awesome-app

# Existing domains (will be checked as-is)  
google.com
github.io

# Empty lines and comments are ignored
awesome-domain    # Inline comments supported
```

---

## 🚀 Performance

Domain Check v0.4.0 delivers significant performance improvements through:

- **Smart Concurrency**: Configurable concurrent processing (default: 10, max: 100)
- **Protocol Optimization**: RDAP-first with intelligent WHOIS fallback
- **Connection Reuse**: HTTP client connection pooling
- **Registry-Specific Tuning**: Optimized timeouts per TLD registry
- **Streaming Architecture**: Results available as they complete

---

## 🛠️ Development

### Running Tests

```bash
# Run all tests
cargo test

# Run library tests only
cargo test -p domain-check-lib

# Run CLI tests only  
cargo test -p domain-check
```

### Examples

```bash
# Library examples
cargo run --example basic_check
cargo run --example bulk_processing
cargo run --example streaming_results

# CLI examples
cargo run -- example.com --info
cargo run -- --file examples/domains.txt -t com,org
```

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/saidutt46/domain-check.git
cd domain-check
cargo build
cargo test
```

### Project Structure

```
domain-check/
├── domain-check-lib/          # Core library
│   ├── src/
│   │   ├── lib.rs            # Public API
│   │   ├── checker.rs        # Main domain checker
│   │   ├── protocols/        # RDAP & WHOIS implementations
│   │   ├── types.rs          # Data structures
│   │   └── error.rs          # Error handling
├── domain-check/             # CLI application
│   └── src/
│       └── main.rs           # CLI interface
└── examples/                 # Usage examples
```

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🔗 Links

- **Documentation**: [docs.rs/domain-check-lib](https://docs.rs/domain-check-lib)
- **Crate**: [crates.io/crates/domain-check](https://crates.io/crates/domain-check)
- **Repository**: [github.com/saidutt46/domain-check](https://github.com/saidutt46/domain-check)
- **Issues**: [GitHub Issues](https://github.com/saidutt46/domain-check/issues)

---

*Built with ❤️ in Rust*