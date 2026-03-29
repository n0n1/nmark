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
cd /Users/itelenk0v/org/projects/rmark/nmark
cargo install --path .
```

This installs the binary as `nmark`.

Verify:

```bash
nmark --help
```

## Build a Release Binary

If you only want the compiled executable:

```bash
cd /Users/itelenk0v/org/projects/rmark/nmark
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
