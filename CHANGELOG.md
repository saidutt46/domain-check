# Changelog

## [0.6.0] - 2025-01-XX

### 🚀 Major Release: Configuration Files & Environment Variables

This release introduces comprehensive configuration management, transforming domain-check from a command-line only tool into a fully configurable domain checking platform with persistent settings, custom presets, and environment variable support.

### 🎉 Added

#### **Configuration File Support**
- **TOML configuration files**: Create `.domain-check.toml` for persistent settings
- **Multi-location discovery**: Local (`./.domain-check.toml`), global (`~/.domain-check.toml`), and XDG (`~/.config/domain-check/config.toml`)
- **Hierarchical precedence**: CLI args > environment variables > local config > global config > XDG config > defaults
- **Comprehensive validation**: Clear error messages for invalid configuration values
- **--config flag**: Specify custom configuration file locations

#### **Environment Variable Support**
- **Complete DC_* variable set**: All CLI options available as environment variables
- **Smart validation**: Invalid values logged with warnings, fallback to next precedence level
- **CI/CD integration**: `DC_CONCURRENCY`, `DC_PRESET`, `DC_TLD`, `DC_TIMEOUT`, `DC_BOOTSTRAP`, etc.
- **File path variables**: `DC_CONFIG` and `DC_FILE` for default file locations

#### **Custom TLD Presets**
- **User-defined presets**: Create custom TLD combinations in config files
- **Preset override**: Custom presets take precedence over built-in presets
- **Flexible usage**: Use custom presets with `--preset my_custom` or `DC_PRESET=my_custom`
- **Smart validation**: Custom presets validated with helpful error messages

#### **Enhanced User Experience**
- **Persistent preferences**: Set default concurrency, presets, output formats
- **Reduced typing**: Common settings automatically applied
- **Team collaboration**: Share config files for consistent team settings
- **Better defaults**: Increased default concurrency from 10 to 20 for improved performance

### 🔄 Changed

#### **Configuration Management**
- **Centralized config resolution**: All settings resolved through unified precedence system
- **Improved error handling**: Configuration errors provide actionable feedback with context
- **Smart precedence logic**: CLI arguments only override when explicitly provided by user
- **Enhanced validation**: Comprehensive validation with user-friendly error messages

#### **Performance Improvements**
- **Default concurrency**: Increased from 10 to 20 concurrent requests for better performance
- **Optimized config loading**: Efficient discovery and caching of configuration files
- **Reduced redundancy**: Eliminated duplicate config loading in domain processing

### 📊 Configuration Examples

#### Basic Configuration File
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
my_crypto = ["com", "org", "crypto", "blockchain", "web3"]

[output]
default_format = "pretty"
csv_headers = true
```

#### Environment Variables
```sh
# Set defaults via environment
export DC_CONCURRENCY=50
export DC_PRESET=startup
export DC_PRETTY=true
export DC_BOOTSTRAP=true
# CI/CD integration
DC_TIMEOUT=30s DC_CONCURRENCY=25 domain-check --file domains.txt
```

#### Precedence Examples
- Config file sets `concurrency=25`, but CLI overrides:
  ```sh
  domain-check --concurrency 50 mystartup  # Uses 50
  ```
- Environment variable overrides config file:
  ```sh
  DC_PRESET=enterprise domain-check mystartup  # Uses enterprise preset
  ```
- Custom preset from config file:
  ```sh
  domain-check --preset my_startup mystartup  # Uses custom preset
  ```

🎯 Use Cases Enabled
Developer Workflows
• Project-specific configs: Different settings per project directory
• Personal defaults: Global settings for individual developer preferences
• Team standardization: Shared config files in repositories

Automation & CI/CD
• Environment-driven configuration: Dynamic settings via environment variables
• Docker integration: Configuration via environment in containerized environments
• Script automation: Reduced command-line complexity in automated tools

Enterprise & Teams
• Consistent settings: Team-wide configuration standards
• Custom domain strategies: Organization-specific TLD presets
• Audit trails: Configuration-driven domain checking policies

🔧 Technical Improvements
• Modular configuration system: Clean separation of config sources and validation
• Type-safe parsing: Comprehensive TOML parsing with validation
• Error recovery: Graceful handling of invalid configurations
• Memory efficiency: Optimized config loading and caching

Developer Experience
• Rich error messages: Actionable feedback for configuration issues
• Comprehensive testing: Full test coverage for configuration features
• Documentation: Complete examples and usage patterns

📋 Migration Guide
For Existing CLI Users
✅ Zero breaking changes - All existing commands work unchanged
🆕 New capabilities - Add config files for persistent settings
🔧 Enhanced workflow - Reduce repetitive typing with defaults

Upgrade Examples

# Before: Repetitive commands
domain-check --concurrency 25 --preset startup --pretty mystartup
domain-check --concurrency 25 --preset startup --pretty anotherdomain
# After: One-time config setup
echo '[defaults]
concurrency = 25
preset = "startup"
pretty = true' > .domain-check.toml
# Now simple commands use your preferences
domain-check mystartup
domain-check anotherdomain

🎉 Community Impact
This release addresses the most requested workflow improvements: persistent configuration and reduced command repetition. The configuration system transforms domain-check from a basic CLI tool into a comprehensive domain management platform suitable for individual developers, teams, and enterprise automation.


## [0.5.1] - 2024-06-24

### 🚀 Distribution & Licensing Updates

This release focuses on improved distribution channels and enhanced legal protection for the growing domain-check ecosystem.

### 🎉 Added

#### **Homebrew Package Support**
- **Automated Homebrew formula**: Full integration with Homebrew package manager for macOS users
- **Cross-platform binaries**: Support for both Intel and Apple Silicon Macs
- **Automatic updates**: GitHub Actions workflow automatically updates Homebrew formula on release
- **Simple installation**: `brew tap saidutt46/domain-check && brew install domain-check`

#### **Enhanced Release Automation**
- **SHA256 checksum calculation**: Automatic security verification for all binary releases
- **Multi-architecture support**: Optimized release process for Intel and ARM Macs
- **Formula synchronization**: Seamless integration between releases and package distribution

### 🔄 Changed

#### **License Migration: MIT → Apache 2.0**
- **Enhanced protection**: Apache 2.0 provides stronger patent protection and attribution requirements
- **Industry standard**: Aligns with major open-source projects for better compatibility
- **Copy protection**: Better safeguards against unauthorized code usage without proper attribution
- **All files updated**: License references updated across workspace, documentation, and badges

#### **Infrastructure Improvements**
- **Version synchronization**: Unified version bump across all workspace crates to 0.5.1
- **Badge updates**: Updated README badges to reflect new license and improved visual consistency
- **Distribution readiness**: Enhanced release pipeline for multiple package managers

### 🛡️ Security & Legal

#### **Strengthened Legal Framework**
- **Patent protection**: Apache 2.0 includes explicit patent grants and protections
- **Clear attribution**: Enhanced requirements for derivative works and distribution
- **Enterprise friendly**: Better compatibility with enterprise legal requirements
- **Community protection**: Stronger safeguards for contributors and users

### 📦 Installation Options

#### **Multiple Distribution Channels**
```bash
# Homebrew (NEW!)
brew tap saidutt46/domain-check
brew install domain-check

# Cargo (existing)
cargo install domain-check

# Library
cargo add domain-check-lib
```

## [0.5.0] - 2025-06-15

### 🚀 Major Release: Universal TLD Checking & Smart Presets

This release introduces game-changing functionality for comprehensive domain availability checking, transforming domain-check from a targeted tool into a universal domain exploration platform.

### 🎉 Added

#### **Universal TLD Checking**
- **`--all` flag**: Check domains against all 42+ known TLDs in a single command
- **Intelligent auto-bootstrap**: Automatically enables IANA registry discovery for comprehensive coverage
- **Streaming results**: Real-time domain availability updates as checks complete
- **No artificial limits**: Removed 1000 domain safety restriction - check as many domains as needed

#### **Smart TLD Presets System**
- **`--preset startup`**: Tech-focused TLDs (com, org, io, ai, tech, app, dev, xyz) - 8 TLDs
- **`--preset enterprise`**: Business-focused TLDs (com, org, net, info, biz, us) - 6 TLDs  
- **`--preset country`**: Major country codes (us, uk, de, fr, ca, au, jp, br, in) - 9 TLDs
- **Case-insensitive preset names**: `--preset STARTUP` works the same as `--preset startup`
- **Comprehensive validation**: Clear error messages for invalid preset names with available options

#### **Enhanced User Experience**
- **Professional error reporting**: Intelligent error aggregation with actionable domain-specific summaries
- **Smart error categorization**: Groups timeouts, network errors, and parsing failures separately
- **Informational messaging**: Clear indication of TLD scope ("Checking against all 42 known TLDs")
- **Error truncation**: Shows first 5 failed domains, then "... and X more" for large error sets

#### **Library API Extensions**
- **`get_all_known_tlds()`**: Extract all TLDs with RDAP endpoints (42+ TLDs, sorted)
- **`get_preset_tlds(preset)`**: Access predefined TLD groups programmatically
- **`get_available_presets()`**: List available preset names for validation
- **Enhanced exports**: All new functions available for library integration

#### **Advanced CLI Features**
- **Argument precedence logic**: `-t` (explicit) > `--preset` (curated) > `--all` (comprehensive) > default (.com)
- **Conflict detection**: Prevents ambiguous TLD source combinations with clear error messages
- **Enhanced file processing**: Works seamlessly with new TLD options for bulk operations
- **Performance optimization**: Smart concurrency management for large TLD sets

### 🔄 Changed

#### **Error Handling Revolution**
- **Streaming mode errors**: Brief inline errors (`domain.app (timeout)`) with comprehensive end summary
- **Batch mode errors**: Detailed errors preserved for debugging while maintaining clean aggregation
- **JSON/CSV modes**: Clean structured output with errors properly embedded in data
- **Full domain context**: Error summaries show complete domain names instead of just TLDs

#### **Validation & Safety**
- **Multi-source validation**: Prevents conflicting TLD source arguments (e.g., `--all --preset startup`)
- **Enhanced preset validation**: Immediate feedback for typos with helpful suggestions
- **Bootstrap auto-enable**: Intelligently enables bootstrap for `--all` and large TLD sets
- **Removed arbitrary limits**: Users control their own resource constraints

#### **Performance Improvements**
- **Optimized concurrency**: Enhanced concurrent processing for 40+ simultaneous TLD checks
- **Smart timeout handling**: Registry-specific timeout optimization for better success rates  
- **Efficient error recovery**: Improved fallback logic with minimal performance impact
- **Streamlined validation**: Faster argument processing and validation chains

### 🐛 Fixed

#### **Error Message Quality**
- **Domain context preservation**: Error summaries now show full domain names instead of meaningless TLD duplicates
- **Actionable error reporting**: Users can identify and retry specific failed domains
- **Intelligent error aggregation**: Similar errors grouped logically with smart truncation
- **Clean output separation**: Errors don't interrupt successful result streams

#### **CLI Robustness**
- **Argument validation edge cases**: Comprehensive validation prevents invalid combinations
- **File processing reliability**: Enhanced domain file parsing with better error recovery
- **Bootstrap integration**: Seamless IANA registry integration for unknown TLDs
- **Memory optimization**: Efficient handling of large domain sets with multiple TLDs

### ⚡ Performance Impact

#### **Capability Expansion**
- **Single command scope**: Check 40+ TLDs instead of manually specifying each one
- **Preset efficiency**: 8 TLD startup check vs 40+ individual specifications
- **Bulk operation scaling**: Process hundreds of domains against multiple TLD sets efficiently
- **Real-time feedback**: Streaming results provide immediate value for large operations

#### **Resource Optimization**
- **Smart concurrency**: Automatic rate limiting prevents registry overwhelm
- **Connection reuse**: Efficient HTTP client pooling for multiple registry endpoints
- **Memory efficiency**: Optimized data structures for large result sets
- **Error resilience**: Graceful handling of registry failures without operation termination

### 🔧 Technical Improvements

#### **Library Architecture**
- **Enhanced modularity**: Clean separation between TLD management and domain checking
- **Type safety**: Strong typing for preset names and TLD collections
- **Comprehensive testing**: 25+ new test cases covering all functionality and edge cases
- **Documentation coverage**: Extensive inline documentation with usage examples

#### **CLI Architecture**  
- **Argument parsing**: Robust clap integration with comprehensive validation
- **Output formatting**: Mode-specific formatting (streaming vs batch vs structured)
- **Error propagation**: Clean error handling from library through to user-friendly messages
- **Backward compatibility**: 100% compatibility with existing usage patterns

### 🎯 Use Cases Enabled

#### **Domain Investment**
```bash
# Explore all TLD opportunities for a brand
domain-check "mybrand" --all --streaming --csv > opportunities.csv
```

#### **Startup Domain Search**  
```bash
# Quick startup-focused domain check
domain-check "mystartup" --preset startup --pretty
```

#### **Enterprise Brand Protection**
```bash
# Comprehensive brand monitoring across business TLDs
domain-check --file brand-variations.txt --preset enterprise --json > monitoring.json
```

#### **International Expansion**
```bash
# Check availability across major country markets
domain-check "mycompany" --preset country --info
```

### 📊 Migration Guide

#### **For Existing CLI Users**
✅ **Zero breaking changes** - all existing commands work unchanged
🆕 **New capabilities** - add `--all` or `--preset` for enhanced functionality
🔧 **Enhanced output** - better error messages and progress indicators

#### **For Library Users**
✅ **API stability** - existing functions unchanged
🆕 **New exports** - `get_all_known_tlds()`, `get_preset_tlds()`, `get_available_presets()`
🔧 **Enhanced types** - improved error handling and result structures

#### **Upgrade Examples**
```bash
# Old approach (manual TLD specification)
domain-check myapp -t com,org,io,ai,tech,app,dev,xyz

# New approach (preset)  
domain-check myapp --preset startup

# New capability (comprehensive checking)
domain-check myapp --all
```

### 🎉 Community Impact

This release addresses the most requested feature: **effortless comprehensive domain checking**. Users can now explore domain availability across the entire TLD landscape with a single command, while smart presets provide curated experiences for common scenarios.

The enhanced error reporting transforms domain-check from a basic availability checker into a professional domain management tool suitable for enterprise workflows and bulk operations.

---

## [0.4.0] - 2025-06-28

### 🚀 Major Release: Library-First Architecture

This release transforms Domain Check from a CLI-only tool into a modern, library-first Rust ecosystem with both library and CLI components.

### ⚠️ Breaking Changes
- **Architecture**: Restructured into workspace with `domain-check-lib` (library) + `domain-check` (CLI)
- **Crate Publishing**: Now publishes two crates - library users should depend on `domain-check-lib`
- **Minimum Rust Version**: Updated to Rust 1.70+ for latest async features

### 🎉 Added
- **Library API**: Complete async library for Rust applications with `DomainChecker` struct
- **Multi-Domain Support**: CLI now accepts multiple domain arguments with smart TLD expansion
- **Streaming Output**: Real-time results display for interactive operations
- **Batch Processing**: Efficient bulk domain checking with configurable concurrency
- **Smart Domain Expansion**: Automatic expansion of base names across specified TLDs
- **Alternative Output Modes**: Force streaming (`--streaming`) or batch (`--batch`) modes
- **Enhanced File Processing**: Improved domain file handling with comment support
- **Registry-Specific Timeouts**: Optimized timeouts per TLD registry for better performance
- **Bootstrap Discovery**: IANA bootstrap registry support for unknown TLDs (`--bootstrap`)
- **Detailed Error Messages**: User-friendly error messages with helpful tips and emojis
- **Debug Mode**: Enhanced debugging with protocol-level insights (`--debug`)
- **Domain Validation**: Input validation to prevent invalid domain processing

### 🔄 Changed
- **CLI Input Model**: Now supports multiple positional domain arguments
- **TLD Argument**: Enhanced `-t/--tld` to support both comma-separated and multiple flag formats
- **Output Formatting**: Improved progress indicators and result presentation
- **Concurrency Model**: Unified concurrent processing engine for both library and CLI
- **Error Handling**: Comprehensive error types with better categorization and recovery
- **Protocol Selection**: Smarter RDAP/WHOIS fallback logic with error-specific handling
- **File Format**: Enhanced domain file support with inline comments and validation

### 🐛 Fixed
- **Registry Endpoints**: Updated RDAP endpoints for `.org`, `.info`, and `.io` TLDs
- **Domain Expansion**: Fixed empty string handling in TLD expansion logic
- **Invalid TLD Handling**: Unknown TLDs now return "UNKNOWN" status instead of false "TAKEN"
- **Info Extraction**: Resolved missing registrar information for non-Verisign TLDs
- **Timeout Management**: Fixed timeout issues with slower registry servers
- **Memory Usage**: Optimized memory usage for large domain lists
- **Error Propagation**: Improved error handling in concurrent operations

### ⚡ Performance Improvements
- **3-5x Faster**: Significant speed improvements for multi-domain operations
- **Smart Concurrency**: Registry-aware rate limiting and connection pooling
- **Reduced Latency**: Optimized RDAP request handling with connection reuse
- **Streaming Results**: Results available as they complete instead of batch-only
- **Protocol Optimization**: Faster RDAP parsing and response handling
- **Registry Tuning**: TLD-specific timeout optimizations for better success rates

### 📚 Library Features
- **Async APIs**: Full async/await support with tokio integration
- **Streaming Support**: Real-time result streaming with `futures::Stream`
- **Configurable**: Extensive configuration options via `CheckConfig`
- **Error Recovery**: Comprehensive error types with automatic fallback logic
- **Zero CLI Dependencies**: Pure library with no CLI-specific dependencies
- **Thread Safe**: Safe for use in multi-threaded applications

### 🛠️ Technical Improvements
- **Workspace Architecture**: Clean separation between library and CLI concerns
- **Protocol Modularity**: Isolated RDAP and WHOIS implementations
- **Registry Management**: Centralized registry mappings with easy updates
- **Type Safety**: Strong typing for domain results and configuration
- **Documentation**: Comprehensive docs.rs documentation with examples
- **Testing**: Enhanced test coverage for both library and CLI components

### 📦 CLI Enhancements
- **Backward Compatible**: Existing CLI usage patterns continue to work
- **New Capabilities**: Multi-domain support, streaming output, enhanced file processing
- **Better UX**: Improved progress indicators, error messages, and result formatting
- **Flexible Input**: Mix of FQDNs and base names with intelligent expansion
- **Output Options**: JSON, CSV, and enhanced text formats

### 🔧 Developer Experience
- **Easy Integration**: Simple 3-line integration for basic domain checking
- **Extensive Examples**: Library and CLI usage examples
- **Migration Guide**: Clear upgrade path from v0.3.x
- **CI/CD Ready**: Workspace-aware CI/CD configuration
- **Modular Testing**: Separate test suites for library and CLI components

### 📋 Migration Guide
**For CLI Users:**
- ✅ Existing commands work unchanged
- ✅ All flags and options preserved
- 🆕 New multi-domain support: `domain-check example startup -t com,org`
- 🆕 Enhanced file processing with better validation

**For Library Integration:**
```toml
[dependencies]
domain-check-lib = "0.4.0"
tokio = { version = "1", features = ["full"] }
```

## [0.3.1] - 2025-04-18

### Performance Improvements
- Optimized concurrency for multi-TLD domain checks
- Reduced timeouts from 30s to 3-8s for faster results
- Implemented TLD grouping to prevent rate limiting issues
- Added smart concurrent processing for bulk operations
- Improved streaming of results as they're available
- Enhanced WHOIS fallback to activate faster when RDAP fails

### Fixed
- Resolved performance bottleneck when checking multiple TLDs
- Fixed race conditions in concurrent domain processing
- Corrected domain parsing for multi-level TLDs
- Addressed memory usage issues with shared registry data
- Resolved resource leaks during parallel processing

### Technical Improvements
- Implemented Arc-based sharing of immutable registry data
- Added IANA bootstrap registry caching for faster lookups
- Improved error handling for network failures
- Enhanced timeout management for unreliable endpoints
- Optimized rate limiting strategy based on TLD groups

## [0.3.0] - 2025-04-12

### Added
- Bulk domain checking from text files with `--file` flag
- Concurrency control with new `--concurrency` parameter (default: 10, max: 100)
- Domain limit safeguard (500 domains max) with `--force` flag to override
- Support for comments (#) and empty lines in input files
- Detailed error reporting for invalid domain entries in files
- Automatic TLD assignment for base domains in files:
  - Uses TLDs specified with `--tld` flag
  - Falls back to .com if no TLD specified
- Summary report showing available/taken/unknown counts after bulk operations

### Changed
- Made original domain parameter optional to support file-only mode
- Improved rate limiting for bulk operations
- Enhanced timeout handling for concurrent requests
- Updated terminal output with mode-specific messaging
- Restructured main function for clearer code organization

### Fixed
- Resolved threading issues with console styles in concurrent operations
- Fixed domain validation to properly handle entries from files
- Addressed potential memory usage issues with large input files
- Ensured compatibility with existing UI, JSON, and info flags

### Developer Improvements
- Added error handling for file operations
- Implemented more robust domain parsing for bulk operations
- Improved concurrency management with semaphores
- Enhanced documentation for new features