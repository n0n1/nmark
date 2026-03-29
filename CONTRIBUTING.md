# Contributing

## Overview

This repository is a single Rust binary crate. Core code lives in `src/`, and unit tests live next to the code they cover under `#[cfg(test)]`.

Main modules:

- `src/main.rs`: bootstrap and CLI entrypoint
- `src/app.rs`: orchestration of fetch -> extract -> render -> write
- `src/cli.rs`: CLI parsing
- `src/settings.rs`, `src/inputs.rs`, `src/tomlish.rs`: config and input-file resolution
- `src/http_client.rs`, `src/extractor.rs`, `src/metadata.rs`, `src/frontmatter.rs`, `src/document.rs`: pipeline stages
- `src/error.rs`: top-level error aggregation

## Development

Run these before opening a PR:

```bash
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Useful local commands:

```bash
cargo run -- --help
cargo run -- https://example.com/article
cargo run -- --input download.toml
```

## Style

Follow idiomatic Rust:

- 4-space indentation
- `snake_case` for functions and modules
- `PascalCase` for structs and enums
- explicit, typed errors over stringly-typed failures

Keep modules small and focused. If you change behavior, add or update tests in the same module.

## Tests

Prefer targeted tests for:

- CLI parsing
- config merging
- input loading
- path generation
- error behavior

Test names should describe behavior, for example:

- `loads_toml_urls`
- `rejects_unknown_option`
- `creates_unique_output_filename_when_file_exists`

## Pull Requests

Keep commits short, imperative, and specific, for example:

- `feat: add config-driven http settings`
- `docs: clarify download.toml format`

PRs should include:

- a short summary of behavior changes
- any new CLI or config examples
- confirmation that tests and clippy passed
