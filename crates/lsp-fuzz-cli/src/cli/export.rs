use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use libafl::inputs::Input;
use lsp_fuzz::lsp_input::LspInput;
use tracing::info;

use super::GlobalOptions;

/// Exports the input corpus to a directory
#[derive(Debug, clap::Parser)]
pub(super) struct ExportCommand {
    /// The path to the solution corpus
    #[clap(long, short)]
    input: PathBuf,

    /// The path to the output directory
    #[clap(long, short)]
    output: PathBuf,
}

impl ExportCommand {
    const FILE_NAME_PREFIX: &str = "input_";

    pub(super) fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        let input_files = fs::read_dir(self.input)
            .context("Reading input directory")?
            .filter_map(|it| {
                it.ok().and_then(|it| {
                    it.file_name()
                        .to_str()
                        .is_some_and(|it| it.starts_with(Self::FILE_NAME_PREFIX))
                        .then(|| it.path())
                })
            });
        for input in input_files {
            info!("Processing {}", input.display());
            let output = self.output.join(
                input
                    .file_name()
                    .expect("The input file should have a file name"),
            );
            export_input(&input, &output)
                .with_context(|| format!("Processing {}", input.display()))?;
        }
        Ok(())
    }
}

fn export_input(input: &Path, output_dir: &Path) -> Result<(), anyhow::Error> {
    let input = LspInput::from_file(input).context("Deserializing input")?;
    if fs::exists(output_dir).context("Checking workspace directory")? {
        fs::remove_dir_all(output_dir).context("Removing workspace directory")?;
    }
    let workspace_dir = output_dir.join("workspace");
    fs::create_dir_all(&workspace_dir).context("Creating workspace directory")?;
    input
        .setup_source_dir(&workspace_dir)
        .context("Setting up workspace directory")?;
    let messages = input.request_bytes(&workspace_dir);
    let message_file = output_dir.join("requests");
    let message_file = File::create(message_file).context("Creating message file")?;
    let mut writer = BufWriter::new(message_file);
    writer
        .write_all(&messages)
        .context("Writing to message file")?;
    Ok(())
}
