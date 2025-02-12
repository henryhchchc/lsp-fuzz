use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use libafl::inputs::Input;
use lsp_fuzz::lsp_input::LspInput;
use rayon::iter::{ParallelBridge, ParallelIterator};
use tracing::info;

use crate::cli::reproduce::repdoruce;
use crate::cli::GlobalOptions;

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct ReproduceAll {
    /// The path to the directory containing the fuzz solutions.
    #[clap(long, short)]
    solution_dir: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,

    /// The path to the output file.
    #[clap(long, short)]
    output_file: PathBuf,
}

impl ReproduceAll {
    pub fn run(self, _global_opttions: GlobalOptions) -> anyhow::Result<()> {
        let input_files = self
            .solution_dir
            .read_dir()
            .context("Reading solution directory")?
            .map(Result::unwrap)
            .filter(|it| {
                it.metadata().is_ok_and(|it| it.is_file())
                    && it.file_name().to_string_lossy().starts_with("input_")
            })
            .map(|it| it.path());

        let reproduction_infos: Vec<_> = input_files
            .par_bridge()
            .map(|input_file| {
                let input_id = input_file
                    .file_name()
                    .expect("We have checked that it is a file")
                    .to_str()
                    .context("The file name is not valid UTF-8")?
                    .to_owned();
                let lsp_input = LspInput::from_file(&input_file).context("Loading input file")?;
                info!("Reproducing crash for input {}", input_id);
                repdoruce(input_id, lsp_input, &self.target_executable)
                    .with_context(|| format!("Reproducing crash for {}", input_file.display()))
            })
            .filter_map(|it| it.unwrap())
            .collect();

        let mut output_file = File::create(&self.output_file).context("Creating output file")?;
        serde_json::to_writer(&mut output_file, &reproduction_infos)
            .context("Writing output file")?;
        Ok(())
    }
}
