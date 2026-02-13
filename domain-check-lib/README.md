# domain-check-lib

**A fast, robust Rust library for checking domain availability using RDAP and WHOIS protocols**

[![Crates.io](https://img.shields.io/crates/v/domain-check-lib.svg)](https://crates.io/crates/domain-check-lib)
[![Documentation](https://docs.rs/domain-check-lib/badge.svg)](https://docs.rs/domain-check-lib)
[![Downloads](https://img.shields.io/crates/d/domain-check-lib.svg)](https://crates.io/crates/domain-check-lib)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

---

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
domain-check-lib = "0.6.0"
tokio = { version = "1", features = ["full"] }
```

**Basic Example:**

```rust
use domain_check_lib::DomainChecker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    let result = checker.check_domain("example.com").await?;
    
    match result.available {
        Some(true) => println!("{} is AVAILABLE", result.domain),
        Some(false) => println!("{} is TAKEN", result.domain),
        None => println!("{} status is UNKNOWN", result.domain),
    }
    
    Ok(())
}
```

---

## ✨ Key Features

🦀 **Pure Async Rust** - Built with tokio for high performance  
🌐 **Dual Protocol Support** - RDAP-first with WHOIS fallback  
⚡ **Concurrent Processing** - Check multiple domains simultaneously  
🎯 **30+ TLD Mappings** - Accurate results across major registries  
🛡️ **Robust Error Handling** - Comprehensive error types with recovery  
📊 **Detailed Information** - Extract registrar, dates, and status codes  
🔄 **Streaming Support** - Real-time results for bulk operations  

---

## 📚 Usage Examples

### Single Domain Check

```rust
use domain_check_lib::{DomainChecker, CheckConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    
    let result = checker.check_domain("google.com").await?;
    
    println!("Domain: {}", result.domain);
    println!("Available: {:?}", result.available);
    println!("Method used: {}", result.method_used);
    
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
        .with_detailed_info(true);
    
    let checker = DomainChecker::with_config(config);
    let domains = vec![
        "example.com".to_string(),
        "google.org".to_string(),
        "github.io".to_string(),
    ];
    
    let results = checker.check_domains(&domains).await?;
    
    for result in results {
        match result.available {
            Some(true) => println!("✅ {} is available", result.domain),
            Some(false) => println!("❌ {} is taken", result.domain),
            None => println!("❓ {} status unknown", result.domain),
        }
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
    let domains = vec![
        "example.com".to_string(),
        "startup.org".to_string(),
        "mybrand.net".to_string(),
    ];
    
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

### Custom Configuration

```rust
use domain_check_lib::{DomainChecker, CheckConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CheckConfig::default()
        .with_concurrency(50)                          // Max 50 concurrent checks
        .with_timeout(Duration::from_secs(10))         // 10 second timeout
        .with_whois_fallback(true)                     // Enable WHOIS fallback
        .with_bootstrap(true)                          // Use IANA bootstrap
        .with_detailed_info(true);                     // Extract full domain info
    
    let checker = DomainChecker::with_config(config);
    let result = checker.check_domain("example.com").await?;
    
    if let Some(duration) = result.check_duration {
        println!("Checked in {}ms via {}", 
            duration.as_millis(), 
            result.method_used
        );
    }
    
    Ok(())
}
```

### **🆕 TLD Management (v0.5.0)**

The library now provides direct access to TLD knowledge for building domain exploration tools:

```rust
use domain_check_lib::{get_all_known_tlds, get_preset_tlds, get_available_presets};

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get all TLDs with RDAP endpoints
    let all_tlds = get_all_known_tlds();
    println!("Checking across {} TLDs", all_tlds.len()); // ~42 TLDs
    
    // Use curated presets for common scenarios
    let startup_tlds = get_preset_tlds("startup").unwrap();
    println!("Startup TLDs: {:?}", startup_tlds); // 8 tech-focused TLDs
    
    // List available presets
    let presets = get_available_presets();
    println!("Available presets: {:?}", presets); // ["startup", "enterprise", "country"]
    
    Ok(())
}
```

### **Smart Domain Expansion**

Combine TLD management with domain expansion for powerful bulk operations:

```rust
use domain_check_lib::{DomainChecker, get_preset_tlds, expand_domain_inputs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    
    // Define base domain names
    let base_names = vec!["myapp".to_string(), "mystartup".to_string()];
    
    // Expand with startup-focused TLDs
    let startup_tlds = get_preset_tlds("startup");
    let domains = expand_domain_inputs(&base_names, &startup_tlds);
    // Results in: myapp.com, myapp.io, myapp.ai, etc.
    
    // Check all expanded domains
    let results = checker.check_domains(&domains).await?;
    
    // Filter for available domains
    let available: Vec<_> = results
        .iter()
        .filter(|r| r.available == Some(true))
        .collect();
        
    println!("Found {} available domains", available.len());
    
    Ok(())
}
```

🌐 **TLD Management**: Access to 40+ known TLDs and curated presets  
🎯 **Smart Expansion**: Intelligent domain name expansion with preset integration  
📊 **Enhanced Results**: Improved error context and domain information

### **Universal TLD Checking**

```rust
use domain_check_lib::{DomainChecker, get_all_known_tlds};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    
    // Check against all known TLDs
    let all_tlds = get_all_known_tlds();
    let domains = domain_check_lib::expand_domain_inputs(
        &["myapp".to_string()], 
        &Some(all_tlds)
    );
    
    println!("Checking {} domains across all TLDs", domains.len());
    
    let results = checker.check_domains(&domains).await?;
    
    // Analyze results
    let available_count = results.iter().filter(|r| r.available == Some(true)).count();
    let taken_count = results.iter().filter(|r| r.available == Some(false)).count();
    
    println!("Results: {} available, {} taken", available_count, taken_count);
    
    Ok(())
}
```

### File-based Processing

```rust
use domain_check_lib::DomainChecker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = DomainChecker::new();
    
    // Process domains from a file (one domain per line)
    let results = checker.check_domains_from_file("domains.txt").await?;
    
    let available_domains: Vec<_> = results
        .iter()
        .filter(|r| r.available == Some(true))
        .collect();
    
    println!("Found {} available domains out of {}", 
        available_domains.len(), 
        results.len()
    );
    
    for domain in available_domains {
        println!("✅ {}", domain.domain);
    }
    
    Ok(())
}
```

---

## 🔧 Configuration Options

The `CheckConfig` struct provides extensive configuration:

```rust
use domain_check_lib::CheckConfig;
use std::time::Duration;

let config = CheckConfig::default()
    .with_concurrency(25)                    // Concurrent requests (1-100)
    .with_timeout(Duration::from_secs(8))    // Per-domain timeout
    .with_whois_fallback(true)               // Enable WHOIS fallback
    .with_bootstrap(false)                   // IANA bootstrap lookup
    .with_detailed_info(true)                // Extract registrar details
    .with_tlds(vec![                         // Default TLDs for expansion
        "com".to_string(),
        "org".to_string(),
        "net".to_string()
    ]);
```

---

## 📊 Data Structures

### DomainResult

```rust
pub struct DomainResult {
    pub domain: String,              // Domain that was checked
    pub available: Option<bool>,     // true = available, false = taken, None = unknown
    pub info: Option<DomainInfo>,    // Detailed registration info (if available)
    pub check_duration: Option<Duration>, // How long the check took
    pub method_used: CheckMethod,    // RDAP, WHOIS, or Bootstrap
    pub error_message: Option<String>, // Error details (if applicable)
}
```

### DomainInfo

```rust
pub struct DomainInfo {
    pub registrar: Option<String>,      // Domain registrar
    pub creation_date: Option<String>,  // When domain was registered
    pub expiration_date: Option<String>, // When domain expires
    pub status: Vec<String>,            // Domain status codes
    pub updated_date: Option<String>,   // Last update date
    pub nameservers: Vec<String>,       // Associated nameservers
}
```

---

## 🌐 Protocol Support

### RDAP (Primary)

- **Modern Protocol**: Structured JSON responses
- **30+ TLD Mappings**: Major registries supported
- **Rich Data**: Registrar info, dates, status codes
- **Performance**: Fast, reliable responses

### WHOIS (Fallback)

- **Universal Coverage**: Works with most TLDs
- **Automatic Parsing**: Intelligent response interpretation
- **Rate Limiting**: Built-in throttling protection
- **Error Recovery**: Smart fallback logic

### Bootstrap Discovery

- **IANA Registry**: Dynamic endpoint discovery
- **Unknown TLDs**: Automatic protocol detection
- **Future Proof**: Adapts to new registries

---

## 🚀 Performance

Domain Check is optimized for high-performance operations:

- **Concurrent Processing**: Configurable parallelism (1-100 concurrent requests)
- **Connection Reuse**: HTTP client connection pooling
- **Smart Timeouts**: Registry-specific timeout optimization
- **Memory Efficient**: Streaming results for large datasets
- **Protocol Selection**: Intelligent RDAP/WHOIS routing

---

## 🛡️ Error Handling

Comprehensive error types with automatic recovery:

```rust
use domain_check_lib::DomainCheckError;

match checker.check_domain("invalid-domain").await {
    Ok(result) => println!("Success: {:?}", result),
    Err(DomainCheckError::InvalidDomain { domain, reason }) => {
        eprintln!("Invalid domain '{}': {}", domain, reason);
    }
    Err(DomainCheckError::NetworkError { message, .. }) => {
        eprintln!("Network error: {}", message);
    }
    Err(DomainCheckError::Timeout { operation, duration }) => {
        eprintln!("Timeout after {:?}: {}", duration, operation);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

---

## 🔗 Related Projects

- **CLI Tool**: [`domain-check`](https://crates.io/crates/domain-check) - Command-line interface
- **Repository**: [GitHub](https://github.com/saidutt46/domain-check) - Source code and issues

---

## 📝 License

This project is licensed under the Apache License, Version 2.0 - see the [LICENSE](../LICENSE) file for details.

---

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](../CONTRIBUTING.md) for details.

---

*Built with ❤️ in Rust*