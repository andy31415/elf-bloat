# elf-bloat

A CLI tool and library for comparing ELF file symbol sizes.

## Overview

`elf-bloat` helps you analyze how code changes affect binary size by comparing the symbols of two ELF files or analyzing a single ELF file. It can pipe the results to external CSV viewers like `visidata` or `csvlens` for interactive analysis.

## Installation

```bash
cargo install --path .
```
Note: If `cargo` is not in PATH, it might be available at `~/.cargo/bin/cargo`.

## Usage

### Analyze a single ELF file

```bash
elf-bloat <FILE>
```

### Compare an ELF file against a baseline

```bash
elf-bloat <FILE> --compare-base <BASE_FILE>
```

### Options

| Option | Default | Description |
|---|---|---|
| `--compare-base <PATH>` | | Baseline build file path for comparison. |
| `--diff-engine <ENGINE>` | `native` | Diff engine to use: `script`, `nm`, `native`, `goblin`. |
| `--viewer <VIEWER>` | `default` | Viewer tool to use: `default`, `vd`, `visidata`, `csvlens`, `custom:<cmd>`. |
| `-l, --log-level <LEVEL>` | `info` | Log verbosity: `off`, `error`, `warn`, `info`, `debug`, `trace`. |

### Diff Engines

- **`native`**: (Default) Uses a native Rust implementation with the `elf` crate to parse symbols.
- **`nm`**: Uses the system's `nm` tool to extract symbols.
- **`goblin`**: Uses the `goblin` crate for ELF parsing.
- **`script`**: Uses an external Python script (requires `uv` and specific environment).

### Viewer Options

- **`default`**: Auto-detect: use `vd` if available, then `csvlens`, otherwise print a plain table.
- **`vd` / `visidata`**: Pipe CSV to `vd -`.
- **`csvlens`**: Pipe CSV to `csvlens` with pre-configured column filters.
- **`custom:<cmd>`**: Pipe CSV to an arbitrary command.

## As a Library

You can also use `elf-bloat` as a library in other Rust projects. The `elf` module provides structured access to symbol data and differences.

```rust
use elf_bloat::elf::parsers::{NativeParser, ElfParser};
use elf_bloat::elf::symbol_diff::compare_symbols;

let parser = NativeParser;
let symbols1 = parser.get_symbols(path1)?;
let symbols2 = parser.get_symbols(path2)?;

let report = compare_symbols(symbols1, symbols2);
// Access report.diffs and report.totals
```
