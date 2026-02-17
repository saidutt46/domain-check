# Automation Guide

This guide focuses on reproducible, non-interactive usage for CI/CD and scripting.

## Recommended Baseline Flags

For deterministic machine workflows:

```bash
domain-check --file domains.txt --batch --json --yes
```

Why:
- `--batch`: stable grouped completion behavior
- `--json`: machine-readable output
- `--yes`: explicit no-prompt intent

## Exit and Policy Strategy

`domain-check` reports domain status in output data. In automation, treat status categories explicitly:
- `available == true`: candidate domain
- `available == false`: taken
- `available == null`: unresolved/unknown, usually retryable

Example policy script:

```bash
domain-check --file domains.txt --batch --json --yes \
  | jq '[.[] | select(.available==null)] | length'
```

## Retry Strategy for `UNKNOWN`

A practical approach:
1. First pass with normal settings.
2. Retry only unknowns after short delay.
3. Escalate persistent unknowns to manual inspection.

## Streaming vs Batch

Use `--streaming` when:
- you want early progress during large runs
- output is consumed live

Use `--batch` when:
- you want predictable end-of-run output
- you archive full JSON/CSV artifacts

## Example CI Snippets

### GitHub Actions step

```yaml
- name: Domain check
  run: |
    domain-check --file domains.txt --batch --json --yes > domain-results.json
```

### Filter available domains

```bash
jq -r '.[] | select(.available==true) | .domain' domain-results.json
```

### Fail when critical domain is unavailable

```bash
jq -e '.[] | select(.domain=="mybrand.com" and .available==true)' domain-results.json > /dev/null
```

## Configuration in CI

You can use environment variables instead of long flags:

```bash
DC_CONCURRENCY=50 \
DC_TIMEOUT=15s \
DC_PRESET=startup \
domain-check --file domains.txt --batch --json --yes
```

Prefer explicit flags for behavior-critical options when reproducibility matters.

## Reproducibility Notes

For strict reproducibility:
- pin CLI version in your toolchain/install process
- record flags used
- capture raw JSON artifacts
- document retry policy for `UNKNOWN`
