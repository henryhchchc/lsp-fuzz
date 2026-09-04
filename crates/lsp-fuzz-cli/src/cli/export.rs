use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use libafl::inputs::Input;
use lsp_fuzz::test_case::{HasWorkspace, LspInput};
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

    #[clap(long)]
    input_prefix: Option<String>,
}

impl ExportCommand {
    pub(super) fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        let input_files = fs::read_dir(self.input)
            .context("Reading input directory")?
            .map(Result::unwrap)
            .filter(|it| {
                it.metadata().is_ok_and(|it| it.is_file())
                    && self
                        .input_prefix
                        .as_ref()
                        .is_none_or(|prefix| it.file_name().to_string_lossy().starts_with(prefix))
            })
            .map(|it| it.path());
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
        .setup_workspace(&workspace_dir)
        .context("Setting up workspace directory")?;
    let requests_dir = output_dir.join("requests");
    fs::create_dir_all(&requests_dir).context("Creating requests dir")?;
    for (idx, json_msg) in input
        .localized_json_rpc_message_sequence(&workspace_dir)
        .enumerate()
    {
        let message_file = requests_dir.join(format!("message_{idx:0>5}"));
        let message_file = File::create(message_file).context("Creating message file")?;
        let mut writer = BufWriter::new(message_file);
        writer
            .write_all(json_msg.to_lsp_payload().as_ref())
            .context("Writing to message file")?;
    }
    Ok(())
}
