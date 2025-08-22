mod cli;
mod fuzzing;

mod language_fragments;

use anyhow::Context;
use clap::Parser;

pub const PROGRAM_NAME: &str = "LSP-Fuzz";

fn main() -> anyhow::Result<()> {
    if rustversion::cfg!(not(nightly)) {
        eprintln!(
            "WARNING: {PROGRAM_NAME} was not compiled with a nightly compiler. \
                      Nightly-only optimizations are not available. \
                      Performance will be lower than expected."
        );
    }
    let cli = cli::Cli::parse();
    cli.run().context("Running CLI")?;
    Ok(())
}
