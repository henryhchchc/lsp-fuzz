# LSPFuzz Tree-sitter grammar preparation

This crate contains the subset of Tree-sitter's grammar generator that LSPFuzz uses to turn `grammar.json` files into derivation rules.

The files under `src/prepare_grammar`, along with `bitvec.rs`, `grammars.rs`, `nfa.rs`, `parse_grammar.rs`, `rules.rs`, and `strpool.rs`, originate from Tree-sitter v0.27.0 (commit `6070dbfefd326bd735e5683eb128cc1b57dad0c0`).
Tree-sitter's MIT license is included in this directory.

Parser-table construction, parser rendering, grammar loading, and node-type generation are intentionally omitted.
`src/lib.rs` is the LSPFuzz-owned facade.
