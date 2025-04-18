# Changelog

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