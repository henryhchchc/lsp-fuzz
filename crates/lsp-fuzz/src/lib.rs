#![warn(missing_debug_implementations, rust_2018_idioms)]

// Nightly features
#![feature(let_chains)]

pub(crate) mod stolen;

pub mod execution;
pub mod lsp_input;
pub mod stages;

pub mod utf8;

pub mod file_system;
pub mod lsp;
pub mod text_document;

pub mod mutators;
pub(crate) mod utils;

pub mod debug;

pub mod afl;
pub mod fuzz_target;
pub(crate) mod macros;
