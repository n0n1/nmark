# Installation

## Requirements

- Rust toolchain with `cargo`

Check:

```bash
rustc --version
cargo --version
```

## Install Into `~/.cargo/bin`

From the repository root:

```bash
cd "$HOME/org/projects/rmark/nmark"
cargo install --path .
```

This installs the binary as `nmark`.

Verify:

```bash
nmark --help
```

## Install MCP Binary For Codex

If you want to use `nmark` as a global MCP tool in Codex, install the `nmark-mcp` binary:

```bash
cd "$HOME/org/projects/rmark/nmark"
cargo install --path . --bin nmark-mcp
```

You can also install both binaries at once:

```bash
cd "$HOME/org/projects/rmark/nmark"
cargo install --path . --bins
```

Verify:

```bash
"$HOME/.cargo/bin/nmark-mcp"
```

It should start and wait for JSON-RPC input on stdin.

Add this to `$HOME/.codex/config.toml`.
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

Restart Codex after editing the config.

## Build a Release Binary

If you only want the compiled executable:

```bash
cd "$HOME/org/projects/rmark/nmark"
cargo build --release
```

The binary will be created at:

`target/release/nmark`

You can copy it into your local bin directory:

```bash
cp target/release/nmark ~/.cargo/bin/
```

## PATH

If `nmark` is not found after installation, add Cargo's bin directory to your shell config:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

For `zsh`, place that line in `~/.zshrc`, then reload the shell:

```bash
source ~/.zshrc
```
