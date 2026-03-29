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
