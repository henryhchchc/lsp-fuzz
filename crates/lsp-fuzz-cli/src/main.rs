mod cli;
mod fuzzing;

mod language_fragments;

use anyhow::Context;
use clap::Parser;

pub const PROGRAM_NAME: &str = "LSP-Fuzz";

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.run().context("Running CLI")?;
    Ok(())
}
