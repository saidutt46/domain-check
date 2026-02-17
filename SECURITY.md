# Security Policy

## Reporting a Vulnerability

Please do not open a public issue for potential security vulnerabilities.

Report privately via:
- GitHub Security Advisories (preferred)
- Or direct maintainer contact if needed

Include:
- Affected versions
- Reproduction details
- Impact assessment
- Any known mitigations

You can expect an initial response within 7 days.

## Supported Versions

Security fixes are prioritized for:

| Version | Supported |
|---|---|
| Latest release | Yes |
| Older releases | Best effort, no guarantees |

## Scope

This policy covers:
- CLI crate: `domain-check`
- Library crate: `domain-check-lib`
- Release/distribution automation in this repository

Out of scope:
- Third-party service outages (registries, DNS infrastructure)
- Vulnerabilities in downstream consumer applications

## Dependency Security

The project uses CI checks for dependency and vulnerability scanning.
Users should still keep dependencies updated and monitor advisories for their pinned versions.
