# domain-check-mcp

MCP (Model Context Protocol) server for domain availability checking. Wraps the [domain-check-lib](https://crates.io/crates/domain-check-lib) library, exposing domain checking tools to any MCP-compatible AI agent or IDE.

## Installation

```bash
cargo install domain-check-mcp
```

Or build from source:

```bash
cargo build --release -p domain-check-mcp
```

## Configuration

### Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "domain-check": {
      "command": "domain-check-mcp"
    }
  }
}
```

### Claude Code

```bash
claude mcp add domain-check -- domain-check-mcp
```

### Custom path

If the binary isn't on your `PATH`, use the full path:

```json
{
  "mcpServers": {
    "domain-check": {
      "command": "/path/to/domain-check-mcp"
    }
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `check_domain` | Check if a single domain is available for registration |
| `check_domains` | Batch check multiple domains concurrently (max 500) |
| `check_with_preset` | Check a base name across all TLDs in a preset |
| `generate_names` | Generate domain name candidates from patterns and affixes |
| `list_presets` | List available TLD presets and their TLDs |
| `domain_info` | Get detailed registration info (registrar, dates, nameservers) |

### Pattern syntax (for `generate_names`)

- `\d` — digit (0-9)
- `\w` — lowercase letter (a-z) or hyphen
- `?` — any of the above

Example: `"app\d\d"` generates `app00` through `app99`.

## Debugging

Set the `RUST_LOG` environment variable for verbose output:

```json
{
  "mcpServers": {
    "domain-check": {
      "command": "domain-check-mcp",
      "env": {
        "RUST_LOG": "domain_check_mcp=debug"
      }
    }
  }
}
```

Logs go to stderr (stdout is reserved for MCP JSON-RPC).

## License

MIT OR Apache-2.0
