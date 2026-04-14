# CLAUDE.md

This file provides guidance for AI assistants working with this repository.

## Commands

```bash
cargo build                  # build
cargo run -- <args>          # run
cargo test                   # run all tests
cargo clippy                 # lint
cargo fmt                    # format
```
Note: If `cargo` is not in PATH, it might be available at `~/.cargo/bin/cargo`.

## Architecture

`elf-size` is a Rust CLI tool and library designed to compare ELF file symbol sizes.

### Project Structure

- **`src/main.rs`**: CLI entry point and orchestration.
- **`src/lib.rs`**: Library entry point, exposes `elf` and `output` modules.
- **`src/output.rs`**: Logic for piping CSV output to external viewers (e.g., `visidata`, `csvlens`).
- **`src/elf/`**: Core logic for ELF parsing and comparison.
    - `diff_engine.rs`: Orchestrates the comparison or analysis flow.
    - `symbols.rs`: Core data structures (`Symbol`, `DiffResult`, `SymbolDiffReport`).
    - `symbol_diff.rs`: Logic for comparing symbols and generating CSV.
    - `parsers/`: Implementations for different ELF parsers (`goblin`, `nm`, `native`).

### Key Design Details

- **Separation of Concerns**: Display logic is separated into `output.rs`, while ELF processing and comparison are in the `elf` module.
- **Re-usability**: The `elf` module is designed to be reusable as a library. `symbol_diff::compare_symbols` returns structured data (`SymbolDiffReport`) instead of just formatted CSV.
- **Diff Engines**: Supports multiple engines for symbol extraction: `nm` (system tool), `native` (using `elf` crate), and `goblin`.
