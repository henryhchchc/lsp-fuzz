use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use libafl::inputs::Input;
use lsp_fuzz::lsp_input::LspInput;
use tracing::info;

use crate::cli::GlobalOptions;
use crate::cli::reproduce::repdoruce;

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct ReproduceOne {
    /// The path to the input file.
    #[clap(long, short)]
    input_file: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_args: Vec<String>,

    /// The path to the output file.
    #[clap(long, short)]
    output_file: PathBuf,
}

impl ReproduceOne {
    pub fn run(self, _global_opttions: GlobalOptions) -> anyhow::Result<()> {
        let input_id = self
            .input_file
            .file_name()
            .expect("We have checked that it is a file")
            .to_str()
            .context("The file name is not valid UTF-8")?
            .to_owned();
        let lsp_input = LspInput::from_file(&self.input_file).context("Loading input file")?;
        info!("Reproducing crash for input {}", input_id);
        let result = repdoruce(
            input_id,
            lsp_input,
            &self.target_executable,
            &self.target_args,
        )
        .with_context(|| format!("Reproducing crash for {}", self.input_file.display()))?;

        if let Some(reproduction_info) = result {
            let mut output_file =
                File::create(&self.output_file).context("Creating output file")?;
            serde_json::to_writer(&mut output_file, &reproduction_info)
                .context("Writing output file")?;
        }

        Ok(())
    }
}
