# FAQ

## Why does a domain show as `UNKNOWN`?

`UNKNOWN` typically means the check could not be completed reliably, for example:
- timeout
- temporary network issues
- registry response differences
- parsing limitations on specific WHOIS servers

Try again with:

```bash
domain-check example.com --debug
```

For automation, treat `UNKNOWN` as retryable unless your policy says otherwise.

## RDAP vs WHOIS: which result is authoritative?

`domain-check` is RDAP-first and uses WHOIS as fallback.
- RDAP is structured and generally easier to parse correctly.
- WHOIS is less standardized and varies by registry.

In borderline cases, retry and validate against registry-native tools if needed.

## What does `--no-bootstrap` do?

It disables IANA bootstrap discovery and restricts checks to hardcoded TLD support.
Use it for:
- offline or constrained environments
- deterministic behavior where external bootstrap fetch is undesired

## Why is docs.rs showing an older version than the repository?

docs.rs reflects published crate versions, not unreleased `main` branch changes.
Use the repository docs for latest in-progress behavior, and docs.rs for latest published release behavior.

## How do I avoid interactive prompts in CI?

Use non-interactive-friendly flags:

```bash
domain-check --file domains.txt --batch --json --yes
```

Non-TTY environments already avoid confirmation prompts, but explicit flags are safer for reproducibility.

## JSON or CSV for automation?

- Use JSON when you need full metadata and robust parsing.
- Use CSV for lightweight spreadsheet/report pipelines.

If schema stability matters, pin tool version and validate expected fields in your pipeline.

## How can I speed up large checks?

- Increase concurrency (`--concurrency 25` to `--concurrency 75`, based on environment).
- Use `--streaming` for early results on long runs.
- Use `--batch --json` for stable downstream processing.
- Use presets to avoid unnecessary TLD scope expansion.

## Can I use this as a Rust library?

Yes. Use `domain-check-lib` for direct integration:
- batch checks
- streaming checks
- config-driven behavior
- domain generation and expansion helpers

See: `domain-check-lib/README.md`.
