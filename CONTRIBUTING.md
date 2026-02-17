# Contributing to domain-check

Thanks for contributing.

This repository contains:
- `domain-check` (CLI crate)
- `domain-check-lib` (library crate)

## Development Setup

Requirements:
- Rust (stable)
- `cargo`
- Optional: `gh` for PR workflow

Clone and verify:

```bash
git clone https://github.com/saidutt46/domain-check.git
cd domain-check
cargo check --workspace
```

## Local Commands

```bash
# Build
cargo build --workspace

# Test
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::uninlined_format_args

# Format check
cargo fmt --all --check
```

## Branch and PR Workflow

- Do not work directly on `main`.
- Create a focused branch (`fix/*`, `feat/*`, `docs/*`, `chore/*`).
- Keep changes scoped; separate refactors from behavior changes.
- Update docs when user-facing behavior changes.
- Add or update tests for behavior changes.

Suggested flow:

```bash
git checkout -b fix/short-description
# make changes
cargo test --workspace
git commit -m "fix: concise summary"
# open PR
```

## Documentation Standards

When editing docs:
- Keep `README.md` as a concise landing page.
- Put deep detail in `docs/CLI.md`, `docs/EXAMPLES.md`, and focused docs under `docs/`.
- Prefer runnable, copy-paste examples.
- Keep version references aligned with current release.

## Commit Message Style

Use conventional prefixes where possible:
- `feat:` new functionality
- `fix:` bug fix
- `docs:` documentation updates
- `refactor:` non-functional code cleanup
- `test:` test-only changes
- `chore:` maintenance

## Reporting Bugs and Requesting Features

Open a GitHub issue with:
- Reproduction steps
- Expected vs actual behavior
- Platform details (OS, Rust version, install method)
- Relevant command and flags

## Release Notes

User-facing changes should include changelog updates in `CHANGELOG.md`.

## Code of Conduct

Be respectful and constructive in issues and PRs.
