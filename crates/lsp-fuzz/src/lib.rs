#![warn(missing_debug_implementations, rust_2018_idioms)]

pub(crate) mod stolen;

pub mod afl;
pub mod corpus;
pub mod debug;
pub mod execution;
pub mod file_system;
pub mod fuzz_target;
pub mod lsp;
pub mod lsp_input;
pub(crate) mod macros;
pub mod mutators;
pub mod stages;
pub mod text_document;
pub mod utf8;
pub(crate) mod utils;
