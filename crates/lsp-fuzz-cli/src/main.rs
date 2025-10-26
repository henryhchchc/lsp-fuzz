mod cli;
mod fuzzing;

mod language_fragments;

use anyhow::Context;
use clap::Parser;

pub const PROGRAM_NAME: &str = "LSP-Fuzz";

fn main() -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        eprintln!("This is a debug build of {PROGRAM_NAME}.");
        eprintln!("Please use a release build for better performance.");
    }

    let cli = cli::Cli::parse();
    cli.run().context("Running CLI")?;
    Ok(())
}
