use std::{fs::File, path::PathBuf};

use anyhow::Context;
use lsp_fuzz::lsp_input::LspInput;

use crate::{cli::GlobalOptions, fuzzing::FuzzerStateDir};

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct CorpusCoverage {
    /// Directory containing the fuzzer states.
    #[clap(long)]
    state: FuzzerStateDir,

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

impl CorpusCoverage {
    pub fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        let input_file_names = self
            .state
            .solution_dir()
            .read_dir()
            .context("Reading solution directory")?
            .map(Result::unwrap)
            .filter(|it| {
                it.metadata().is_ok_and(|it| it.is_file())
                    && it
                        .file_name()
                        .to_string_lossy()
                        .starts_with(LspInput::NAME_PREFIX)
            })
            .map(|it| {
                it.file_name()
                    .into_string()
                    .expect("File name should be valid UTF-8")
            });

        Ok(())
    }
}
