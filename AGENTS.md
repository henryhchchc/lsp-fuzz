# Project Information of LSPFuzz

## What This Project Is

LSPFuzz is a grey-box hybrid fuzzer for language servers (LSP servers), built on top of [LibAFL](https://github.com/AFLplusplus/LibAFL).
It generates test cases that consist of a virtual workspace (source files) plus a sequence of LSP messages, then feeds them to an AFL++-instrumented LSP server binary to find crashes.

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
cargo fmt --all

# Check spelling (uses typos and codebook)
typos
```

The toolchain is pinned to stable Rust (see `rust-toolchain.toml`).
The workspace uses Rust 2024 edition.

## Workspace Structure

Four crates under `crates/`:

| Crate | Role |
|---|---|
| `lsp-fuzz` | Core library: all fuzzing logic, types, and algorithms |
| `lsp-fuzz-cli` | Binary: CLI front-end that wires the library into a runnable fuzzer |
| `lsp-fuzz-grammars` | Tree-sitter grammar wrappers for all supported languages |
| `lsp-fuzz-tree-sitter-grammar` | Embedded Tree-sitter grammar compiler used for grammar-driven generation |

## Core Architecture

### Test Cases (`lsp-fuzz/src/test_case/`)

The fuzzer's input type is `LspInput`, which contains:

- `workspace: FileSystemDirectory<WorkspaceEntry>` — a virtual in-memory file system tree.
  Each entry is either a `SourceFile(TextDocument)` (sent to the LSP via `textDocument/didOpen`) or a `Skeleton(Vec<u8>)` (written to disk but not opened).
- `messages: LspMessageSequence` — the sequence of LSP requests/notifications to send after workspace initialization.

When the fuzzer runs a target, `LspInput::message_sequence()` expands the stored input into a full protocol sequence: `Initialize` → `Initialized` → `didOpen` for each source file → stored messages → `Shutdown` → `Exit`.
`LspInput::localized_json_rpc_message_sequence()` owns request IDs and replaces virtual URIs with real `file://` paths at the wire boundary.
Server responses are lifted back into virtual URI form before feedback and generation inspect them.

`test_case::message_generation` owns LSP-aware parameter generation, compositions, server-feedback guidance, and `GeneratorsConfig`.
`test_case::document_mutation` owns the mutators that select a document from a test case and recalibrate dependent LSP positions after an edit.
`lsp_input` no longer exists.

### Text Document Mutation (`lsp-fuzz/src/text_document/`)

`TextDocument` stores source code content alongside a live tree-sitter parse tree and pre-computed metadata (node-type ranges, node signatures for context awareness).
Every edit goes through `GrammarBasedMutation::edit()`, which keeps the parse tree incrementally updated.

Mutations are grammar-guided:

- `ReplaceNodeMutation` — selects a tree-sitter node and replaces it with a newly generated fragment.
- `NodeContentMutation` — mutates the raw bytes of a node's content.
- Node generators: `ChooseFromDerivations` (pick a real code fragment from corpus), `ExpandGrammar` (generate from tree-sitter grammar), `MismatchedNode` (intentionally wrong type), `EmptyNode`.

### LSP Protocol (`lsp-fuzz/src/lsp/`)

`LspMessage` is a large enum covering all LSP requests and notifications, generated via the `lsp_messages!` macro in `macros.rs`.
This module is deliberately independent of test-case storage and URI localization.
It owns typed LSP messages, metadata, response decoding, and JSON-RPC framing only.

### Execution (`lsp-fuzz/src/execution/`)

`LspExecutor` wraps a custom fork server (`NeoForkServer`) that speaks the AFL++ fork server protocol.
Input is delivered via shared memory (AFL persistent mode).
The executor also:

- Captures stdout for LSP response parsing (fed to `LspOutputObserver`).
- Reads ASAN log files per child PID and feeds them to `AsanBacktraceObserver`.
- Detects persistent mode and defer-fork-server mode by scanning the binary for AFL++ signatures.

### Language Grammars (`lsp-fuzz-grammars/`)

`Language` enum lists all supported languages (C, C++, JavaScript, Ruby, Rust, TOML, LaTeX, BibTeX, Verilog, Solidity, MLIR, QML).
The `language_data.rs` and `language.rs` files map each variant to its tree-sitter parser and LSP language ID.
Some grammars use forked upstream repos (hosted under `github.com/henryhchchc`).

### CLI (`lsp-fuzz-cli/src/cli/`)

Five subcommands:

- `fuzz` — main fuzzing loop (single process, no multi-core orchestration yet)
- `mine-code-fragments` — static analysis phase that extracts real code snippets from a directory of source files for use in `ChooseFromDerivations`
- `export` — converts binary corpus entries to human-readable workspace + request files
- `reproduce-one` / `reproduce-all` — replay individual crash inputs

### Corpus Serialization

`LspInput` is serialized to disk in CBOR format (via `ciborium`), with zstd compression available.
Corpus files are named `id_<N>_time_<T>_exec_<E>` (set by `TestCaseFileNameFeedback`).

## Key Design Notes

- **`lsp-fuzz://` URI scheme** is internal to `test_case`.
  Never store real workspace paths in `LspInput`; localize only through its wire-sequence API.
- **Uniform seed workspaces:** all languages use a single `main.<extension>` source file.
  Do not restore the Rust-specific `rust-project.json` setup without an explicit portability design.
- **Boundary direction:** `lsp` and `text_document` must not import `test_case`; test-case logic may depend on both.
  Execution depends on the test-case workspace contract, never the reverse.
- **Focused support modules:** add LibAFL helpers to `libafl_support`, URI helpers to `test_case::uri`, and LSP/Tree-sitter conversions to `text_document::conversions`; do not recreate `utils.rs`.
- **`stolen/`** contains code adapted from upstream tree-sitter's grammar compiler to drive grammar-based generation without shelling out to Node.js.
- The workspace dependency `lsp-types` is patched to a custom fork (`github.com/henryhchchc/lsp-types`) — check that fork when debugging LSP type issues.
- Debug builds print a warning and are significantly slower; always use `--release` for benchmarking or actual fuzzing runs.
- Logging is configured via `RUST_LOG` env var (default level: `info`).
