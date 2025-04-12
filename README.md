# Domain Check

A fast command-line tool to check domain availability using RDAP with support for WHOIS fallback.

## Features

- Check domain availability using RDAP protocol
- Dynamic discovery of RDAP endpoints via IANA bootstrap
- WHOIS fallback when RDAP is unavailable
- Detailed domain information (registrar, dates, status)
- Support for multiple TLDs
- Interactive terminal UI
- JSON output support

## Installation

```bash
cargo install domain-check
```

## Usage

# Basic usage
domain-check example

# Check multiple TLDs
domain-check example -t com org net

# Check with detailed info
domain-check example -i

# Check with bootstrap registry for unknown TLDs
domain-check example -b

# Check with WHOIS fallback
domain-check example -w

# Use interactive UI
domain-check example -u

# All features with pretty output and JSON
domain-check example -t com org io -i -b -w -p -j
