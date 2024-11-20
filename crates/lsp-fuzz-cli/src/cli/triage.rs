use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::Context;
use libafl::inputs::Input;
use lsp_fuzz::lsp_input::LspInput;

use super::GlobalOptions;

/// Triages the input.
#[derive(Debug, clap::Parser)]
pub(super) struct TriageCommand {
    /// The path to the input.
    #[clap(long, short)]
    input: PathBuf,

    /// The language to use for parsing the source files
    #[clap(long, short)]
    workspace: PathBuf,
}

impl TriageCommand {
    pub(super) fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        let input = LspInput::from_file(self.input).context("Deserializing input")?;

        if fs::exists(&self.workspace).context("Checking workspace directory")? {
            fs::remove_dir_all(&self.workspace).context("Removing workspace directory")?;
        }
        fs::create_dir_all(&self.workspace).context("Creating workspace directory")?;

        input
            .setup_source_dir(&self.workspace)
            .context("Setting up workspace directory")?;

        let messages = input.request_bytes(&self.workspace);
        let mut message_file = self
            .workspace
            .parent()
            .unwrap()
            .join(self.workspace.file_name().unwrap());
        message_file.set_extension("msg");
        let message_file = File::create(message_file).context("Creating message file")?;
        let mut writer = BufWriter::new(message_file);
        writer
            .write_all(&messages)
            .context("Writing to message file")?;

        Ok(())
    }
}
