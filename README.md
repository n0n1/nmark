# nmark

`nmark` is a small Rust CLI that downloads article pages, extracts the main readable content, converts it to Markdown, and optionally prepends YAML frontmatter.

It supports both single-URL usage and batch downloads from `.txt` or `.toml` files.

## Features

- Download and convert a single article URL
- Batch process URL lists from `download.toml`, other `*.toml`, or `*.txt`
- Add YAML frontmatter with timestamp, source, author, and tags when available
- Read config from local and global `config.toml`
- Avoid overwriting files when multiple articles resolve to the same title

## Build

```bash
cargo build
```

## Usage

```bash
cargo run -- https://example.com/article
cargo run -- --input download.toml
cargo run -- urls.txt --output-dir content
cargo run -- --help
```

## Codex Plugin

This repository now includes a repo-local Codex plugin at `plugins/nmark/`.

- MCP server binary: `cargo run --quiet --bin nmark-mcp`
- Plugin manifest: `plugins/nmark/.codex-plugin/plugin.json`
- MCP server config: `plugins/nmark/.mcp.json`
- Marketplace entry: `.agents/plugins/marketplace.json`

The plugin exposes one MCP tool:

- `nmark_convert`: fetch an article URL and return readable Markdown, optionally saving it to a file

### MCP Setup In Codex

`nmark` is configured as a repo-local Codex plugin. No global `$HOME/.codex/config.toml` entry is required for this repository.

1. Open this repository in Codex.
2. Make sure Rust dependencies are available and the project builds.
3. Reload or restart Codex so it re-reads `.agents/plugins/marketplace.json`.
4. Codex will start the MCP server using `plugins/nmark/.mcp.json`, which runs:

```bash
cargo run --quiet --bin nmark-mcp
```

If the tool does not appear, verify that `cargo run --quiet --bin nmark-mcp` starts successfully in the repository root.

### Global MCP Setup

If you want `nmark` to be available in every Codex session, install the MCP binary and register it in `$HOME/.codex/config.toml`.

Install the binary:

```bash
cargo install --path . --bin nmark-mcp
```

Add this entry to `$HOME/.codex/config.toml`.
Use absolute paths here. Do not rely on `$HOME` expansion inside MCP config values.

```toml
[mcp_servers.nmark]
command = "/Users/your-username/.cargo/bin/nmark-mcp"
args = []
cwd = "/Users/your-username/path_to_nmark"
startup_timeout_sec = 20
tool_timeout_sec = 60
required = false
```

Restart Codex after updating the config.

CLI summary:

```text
nmark [<url-or-input>] [--url <url> | --input <path>] [--output-dir <dir>] [--stdout] [--no-frontmatter]
```

## Configuration

Configuration is resolved in this order:

1. CLI arguments
2. `./config.toml`
3. `~/.config/rmark/config.toml`
4. built-in defaults

`config.toml` is for settings only. It does not define download sources.

If `output_dir` is not set anywhere, files are written to the current working directory.

Example `config.toml`:

```toml
output_dir = "articles"
stdout = false
include_frontmatter = true

[http]
request_timeout_sec = 20
connect_timeout_sec = 10
max_redirects = 10
upgrade_insecure_requests = true
```

Example `download.toml`:

```toml
urls = [
  "https://example.com/article-1",
  "https://example.com/article-2",
]
```

Example `urls.txt`:

```text
https://example.com/article-1
https://example.com/article-2
```

## Development

```bash
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Core code lives in `src/`, with modules for CLI parsing, settings resolution, input loading, HTTP, extraction, metadata parsing, frontmatter generation, and document writing.

See [config.md](config.md) for the full configuration reference.
