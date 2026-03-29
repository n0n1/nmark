# Configuration Reference

## Overview

`nmark` uses two different kinds of files:

- `config.toml`: runtime settings only
- `download.toml` or another input file: URLs to download

This separation is strict. `config.toml` does not define download sources.

## Settings Files

Settings are resolved in this order:

1. CLI flags
2. local `./config.toml`
3. global `~/.config/rmark/config.toml`
4. built-in defaults

If `XDG_CONFIG_HOME` is set, the global config path becomes:

`$XDG_CONFIG_HOME/rmark/config.toml`

## Supported Keys In `config.toml`

```toml
output_dir = "articles"
stdout = false
include_frontmatter = true

[http]
connect_timeout_sec = 10
request_timeout_sec = 20
max_redirects = 10
user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36"
accept = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
accept_language = "en-US,en;q=0.9,ru;q=0.8"
upgrade_insecure_requests = true
```

Supported keys:

- `output_dir = "articles"`
  Output directory for generated Markdown files.
  If omitted, files are written to the current working directory.

- `stdout = true|false`
  If `true`, write Markdown to stdout instead of creating files.

- `write_to_stdout = true|false`
  Alias for `stdout`.

- `include_frontmatter = true|false`
  Controls YAML frontmatter generation.

- `frontmatter = true|false`
  Alias for `include_frontmatter`.

## HTTP Settings

HTTP client settings live under `[http]` in `config.toml`.

```toml
[http]
connect_timeout_sec = 10
request_timeout_sec = 20
max_redirects = 10
user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36"
accept = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
accept_language = "en-US,en;q=0.9,ru;q=0.8"
upgrade_insecure_requests = true
```

Supported keys:

- `connect_timeout_sec = 10`
  TCP connect timeout in seconds.

- `request_timeout_sec = 20`
  Full request timeout in seconds.

- `max_redirects = 10`
  Maximum number of followed redirects.

- `user_agent = "..."`
  Value used for the `User-Agent` header.

- `accept = "..."`
  Value used for the `Accept` header.

- `accept_language = "..."`
  Value used for the `Accept-Language` header.

- `referer = "..."`
  Optional explicit `Referer` header. If omitted, `nmark` derives a same-origin referer like `https://example.com/`.

- `upgrade_insecure_requests = true|false`
  Controls whether `Upgrade-Insecure-Requests: 1` is sent.

Built-in defaults:

- `connect_timeout_sec = 10`
- `request_timeout_sec = 20`
- `max_redirects = 10`
- `user_agent = "nmark/<version>"`
- `user_agent = "Mozilla/5.0 ... Chrome/136.0.0.0 Safari/537.36"`
- `accept = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"`
- `accept_language = "en-US,en;q=0.9,ru;q=0.8"`
- `referer = derived from request URL origin`
- `upgrade_insecure_requests = true`

## Recommended Profiles

These are not built-in named profiles. They are example presets you can copy into `config.toml`.

### `default`

Balanced settings for normal use:

```toml
[http]
connect_timeout_sec = 10
request_timeout_sec = 20
max_redirects = 10
```

### `fast`

Use this for quick manual runs where fast failure is better than waiting:

```toml
[http]
connect_timeout_sec = 3
request_timeout_sec = 8
max_redirects = 5
```

Tradeoff:

- faster feedback
- more likely to fail on slow sites or unstable networks

### `slow-sites`

Use this for heavy pages, long redirect chains, or slower connections:

```toml
[http]
connect_timeout_sec = 15
request_timeout_sec = 45
max_redirects = 15
```

Tradeoff:

- more tolerant of slow servers
- longer wait before failure

## Unsupported Keys In `config.toml`

These should not be used in `config.toml`:

- `url = "..."`
- `input = "..."`
- `urls = [...]`

If you need download sources, use CLI arguments or an input file.

## Input Sources

`nmark` accepts sources in this order:

1. positional URL or input path
2. `--url <url>`
3. `--input <path>`
4. fallback `./download.toml` if no source was provided

Examples:

```bash
nmark https://example.com/article
nmark --input download.toml
nmark urls.txt
```

## Supported Input File Formats

### `download.toml`

Accepted forms:

```toml
urls = ["https://example.com/a", "https://example.com/b"]
```

```toml
[download]
urls = ["https://example.com/a", "https://example.com/b"]
```

Single-URL forms are intentionally not supported in `download.toml`.
For one article, use direct CLI input instead:

```bash
nmark https://example.com/article
```

### `.txt`

One URL per line:

```text
https://example.com/a
https://example.com/b
```

Rules:

- empty lines are ignored
- lines starting with `#` are ignored

## CLI Overrides

CLI flags override values from `config.toml`:

- `--output-dir <dir>`
- `--stdout`
- `--no-frontmatter`

Run:

```bash
nmark --help
```

for the current CLI summary and examples.
