mod cli;

use anyhow::Context;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.run().context("Running CLI")?;
    Ok(())
}
