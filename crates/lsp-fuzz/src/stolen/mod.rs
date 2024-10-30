//! Module for the code stolen from other projects.
//!
//! This module contains code that was taken from other projects and adapted to the needs of this project.
//! For example, some usefule APIs are private in the original project, so they are copied here and made public.
//!
//! The sub-module `upstream` contains the original code that was copied.
//! Other sub-modules contain the APIs that need to be exposed to the rest of the project.

pub(crate) mod tree_sitter_generate;

mod upstream;
