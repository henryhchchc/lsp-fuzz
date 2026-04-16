# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

## What This Project Is

LSPFuzz is a grey-box hybrid fuzzer for Language Server Protocol (LSP) servers, built on top of [LibAFL](https://github.com/AFLplusplus/LibAFL). It generates test cases that consist of a virtual workspace (source files) plus a sequence of LSP messages, then feeds them to an AFL++-instrumented LSP server binary to find crashes.

## Commands

```bash
# Build (debug)
cargo build

# Build (release — required for actual fuzzing)
cargo build --release

# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p lsp-fuzz

# Run a single test
cargo test -p lsp-fuzz text_doc_lines

# Lint
cargo clippy --workspace

# Format
cargo fmt --workspace

# Check spelling (uses typos and codebook)
typos
```

The toolchain is pinned to stable Rust (see `rust-toolchain.toml`). The workspace uses Rust 2024 edition.

## Workspace Structure

Three crates under `crates/`:

| Crate | Role |
|---|---|
| `lsp-fuzz` | Core library: all fuzzing logic, types, and algorithms |
| `lsp-fuzz-cli` | Binary: CLI front-end that wires the library into a runnable fuzzer |
| `lsp-fuzz-grammars` | Tree-sitter grammar wrappers for all supported languages |

## Core Architecture

### Input Representation (`lsp-fuzz/src/lsp_input/`)

The fuzzer's input type is `LspInput`, which contains:

- `workspace: FileSystemDirectory<WorkspaceEntry>` — a virtual in-memory file system tree. Each entry is either a `SourceFile(TextDocument)` (sent to the LSP via `textDocument/didOpen`) or a `Skeleton(Vec<u8>)` (written to disk but not opened, e.g., `rust-project.json`).
- `messages: LspMessageSequence` — the sequence of LSP requests/notifications to send after workspace initialization.

When the fuzzer runs a target, `LspInput::message_sequence()` expands the stored input into a full protocol sequence: `Initialize` → `Initialized` → `didOpen` for each source file → stored messages → `Shutdown` → `Exit`. The virtual `lsp-fuzz://` URI scheme is replaced with real `file://` paths at execution time via `localize_json_value`.

### Text Document Mutation (`lsp-fuzz/src/text_document/`)

`TextDocument` stores source code content alongside a live tree-sitter parse tree and pre-computed metadata (node-type ranges, node signatures for context awareness). Every edit goes through `GrammarBasedMutation::edit()`, which keeps the parse tree incrementally updated.

Mutations are grammar-guided:

- `ReplaceNodeMutation` — selects a tree-sitter node and replaces it with a newly generated fragment.
- `NodeContentMutation` — mutates the raw bytes of a node's content.
- Node generators: `ChooseFromDerivations` (pick a real code fragment from corpus), `ExpandGrammar` (generate from tree-sitter grammar), `MismatchedNode` (intentionally wrong type), `EmptyNode`.

### LSP Message Generation (`lsp-fuzz/src/lsp/`)

`LspMessage` is a large enum covering all LSP requests and notifications, generated via the `lsp_messages!` macro in `macros.rs`. Parameter generation for each message type is in `lsp/generation/`. The `GeneratorsConfig` struct controls which optional generation strategies are active (context awareness, grammar-ops awareness, server-feedback guidance, invalid position/range injection).

### Execution (`lsp-fuzz/src/execution/`)

`LspExecutor` wraps a custom fork server (`NeoForkServer`) that speaks the AFL++ fork server protocol. Input is delivered via shared memory (AFL persistent mode). The executor also:

- Captures stdout for LSP response parsing (fed to `LspOutputObserver`).
- Reads ASAN log files per child PID and feeds them to `AsanBacktraceObserver`.
- Detects persistent mode and defer-fork-server mode by scanning the binary for AFL++ signatures.

### Language Grammars (`lsp-fuzz-grammars/`)

`Language` enum lists all supported languages (C, C++, JavaScript, Ruby, Rust, TOML, LaTeX, BibTeX, Verilog, Solidity, MLIR, QML). The `language_data.rs` and `language.rs` files map each variant to its tree-sitter parser and LSP language ID. Some grammars use forked upstream repos (hosted under `github.com/henryhchchc`).

### CLI (`lsp-fuzz-cli/src/cli/`)

Five subcommands:

- `fuzz` — main fuzzing loop (single process, no multi-core orchestration yet)
- `mine-code-fragments` — static analysis phase that extracts real code snippets from a directory of source files for use in `ChooseFromDerivations`
- `export` — converts binary corpus entries to human-readable workspace + request files
- `reproduce-one` / `reproduce-all` — replay individual crash inputs

### Corpus Serialization

`LspInput` is serialized to disk in CBOR format (via `ciborium`), with zstd compression available. Corpus files are named `id_<N>_time_<T>_exec_<E>` (set by `TestCaseFileNameFeedback`).

## Key Design Notes

- **`lsp-fuzz://` URI scheme** is an internal virtual scheme used throughout the fuzzer. URIs are "localized" (replaced with real `file://` paths) just before sending to the target, and "lifted" back when parsing server responses. Never hard-code real paths into `LspInput`.
- **`stolen/`** contains code adapted from upstream tree-sitter's grammar compiler to drive grammar-based generation without shelling out to Node.js.
- The workspace dependency `lsp-types` is patched to a custom fork (`github.com/henryhchchc/lsp-types`) — check that fork when debugging LSP type issues.
- Debug builds print a warning and are significantly slower; always use `--release` for benchmarking or actual fuzzing runs.
- Logging is configured via `RUST_LOG` env var (default level: `info`).
