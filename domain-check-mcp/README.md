# domain-check-mcp

<!-- mcp-name: io.github.saidutt46/domain-check -->

A [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server for domain availability checking. Exposes the [domain-check-lib](https://crates.io/crates/domain-check-lib) engine as structured tools for AI coding agents and MCP-compatible clients.

**Protocol**: MCP over stdio (JSON-RPC 2.0)
**Transport**: stdin/stdout
**SDK**: [rmcp](https://crates.io/crates/rmcp) (official Rust MCP SDK)

## Tools

All tools return structured JSON. All tools are **read-only** — no side effects, safe to call without confirmation.

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `check_domain` | Check availability of a single fully-qualified domain name | `domain` (string, required) |
| `check_domains` | Batch check multiple domains concurrently | `domains` (string[], required), `concurrency` (int, optional, default 20, max 500) |
| `check_with_preset` | Check a base name across all TLDs in a named preset | `base_name` (string, required), `preset` (string, required) |
| `generate_names` | Generate domain name candidates from patterns and affixes | `pattern` (string, optional), `base_names` (string[], optional), `prefixes`/`suffixes` (string[], optional), `tlds` (string[], optional) |
| `list_presets` | List all available TLD presets with their TLD lists | _(none)_ |
| `domain_info` | Get detailed registration info: registrar, dates, nameservers, status codes | `domain` (string, required) |

### Tool annotations

| Tool | readOnlyHint | destructiveHint | idempotentHint |
|------|:---:|:---:|:---:|
| `check_domain` | true | false | true |
| `check_domains` | true | false | true |
| `check_with_preset` | true | false | true |
| `generate_names` | true | false | true |
| `list_presets` | true | false | true |
| `domain_info` | true | false | true |

### Pattern syntax (for `generate_names`)

| Wildcard | Expands to | Example |
|----------|-----------|---------|
| `\d` | digits 0-9 | `app\d\d` generates `app00`..`app99` |
| `\w` | lowercase a-z + hyphen | `\w\wai` generates `aaai`..`-zai` |
| `?` | digits + letters + hyphen | `go?` generates `go0`..`go-` |

### Presets

11 built-in presets: `startup`, `popular`, `classic`, `enterprise`, `tech`, `creative`, `ecommerce`, `finance`, `web`, `trendy`, `country`. Use `list_presets` to see full TLD lists.

## Installation

### From crates.io

```bash
cargo install domain-check-mcp
```

### From source

```bash
git clone https://github.com/saidutt46/domain-check.git
cd domain-check
cargo build --release -p domain-check-mcp
# Binary at: target/release/domain-check-mcp
```

### From GitHub releases

Pre-built binaries for Linux (x86_64, musl), macOS (x86_64, aarch64), and Windows are attached to each [GitHub release](https://github.com/saidutt46/domain-check/releases).

## Client configuration

### Claude Code (verified working)

```bash
claude mcp add domain-check -- domain-check-mcp
```

### Claude Desktop

Add to `claude_desktop_config.json` (`~/Library/Application Support/Claude/` on macOS):

```json
{
  "mcpServers": {
    "domain-check": {
      "command": "domain-check-mcp"
    }
  }
}
```

### VS Code / GitHub Copilot

Add to `.vscode/mcp.json` in your workspace:

```json
{
  "servers": {
    "domain-check": {
      "command": "domain-check-mcp",
      "type": "stdio"
    }
  }
}
```

### Cursor

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "domain-check": {
      "command": "domain-check-mcp"
    }
  }
}
```

### Windsurf

Add to `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "domain-check": {
      "command": "domain-check-mcp"
    }
  }
}
```

### JetBrains IDEs

Settings > Tools > AI Assistant > MCP Servers, add:

```json
{
  "servers": {
    "domain-check": {
      "command": "domain-check-mcp",
      "type": "stdio"
    }
  }
}
```

### OpenAI Codex CLI

```bash
codex mcp add domain-check -- domain-check-mcp
```

### Gemini CLI

```bash
gemini mcp add domain-check -- domain-check-mcp
```

### Any MCP client (generic stdio)

The server reads JSON-RPC from stdin and writes to stdout. Point any MCP-compatible client at the `domain-check-mcp` binary with stdio transport.

```json
{
  "command": "domain-check-mcp",
  "transport": "stdio"
}
```

### Custom binary path

If the binary is not on your `PATH`, use the full path in any configuration above:

```json
{
  "command": "/usr/local/bin/domain-check-mcp"
}
```

## Example usage

Once configured, ask your AI agent naturally:

- "Is `coolstartup.com` available?"
- "Check `mybrand` across the startup preset"
- "Generate domain names matching `app\d\d` and check `.com` and `.io`"
- "Get registration details for `example.com`"
- "What TLD presets are available?"

## Debugging

Set `RUST_LOG` for verbose output (logs go to stderr, stdout is reserved for MCP JSON-RPC):

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

### MCP Inspector

Test the server interactively with the [MCP Inspector](https://github.com/modelcontextprotocol/inspector):

```bash
npx @modelcontextprotocol/inspector domain-check-mcp
```

### Manual JSON-RPC

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | domain-check-mcp
```

## Architecture

- Thin wrapper over [domain-check-lib](https://crates.io/crates/domain-check-lib) — same engine as the CLI
- Single `DomainChecker` instance shared across all tool calls (connection pooling)
- Tools that accept custom concurrency/timeout create a temporary checker per call
- All errors returned as tool content (not protocol errors) so agents can read and act on them
- Safety limits: batch max 500 domains, pattern generation max 100,000 names

## Related

- [domain-check](https://crates.io/crates/domain-check) — CLI for humans
- [domain-check-lib](https://crates.io/crates/domain-check-lib) — Rust library
- [MCP specification](https://spec.modelcontextprotocol.io)

## License

MIT OR Apache-2.0
