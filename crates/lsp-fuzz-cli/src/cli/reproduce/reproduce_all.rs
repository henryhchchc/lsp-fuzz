use std::{fs::File, path::PathBuf};

use anyhow::Context;
use libafl::inputs::Input;
use lsp_fuzz::lsp_input::LspInput;
use rayon::iter::{ParallelBridge, ParallelIterator};
use tracing::info;

use crate::cli::{GlobalOptions, reproduce::reproduce};

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct ReproduceAll {
    /// The path to the directory containing the fuzz solutions.
    #[clap(long, short)]
    solution_dir: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_args: Vec<String>,

    /// The path to the output file.
    #[clap(long, short)]
    output_file: PathBuf,

    #[clap(long)]
    no_parallel: bool,
}

impl ReproduceAll {
    pub fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        info!(?self);
        let input_files = self
            .solution_dir
            .read_dir()
            .context("Reading solution directory")?
            .map(Result::unwrap)
            .filter(|it| {
                it.metadata().is_ok_and(|it| it.is_file())
                    && it.file_name().to_string_lossy().starts_with("id_")
            })
            .map(|it| it.path());

        let reproduce_one = |input_file: PathBuf| {
            let input_id = input_file
                .file_name()
                .expect("We have checked that it is a file")
                .to_str()
                .context("The file name is not valid UTF-8")?
                .to_owned();
            let lsp_input = LspInput::from_file(&input_file)
                .with_context(|| format!("Loading input file: {}", input_file.display()))?;
            info!("Reproducing crash for input {}", input_id);
            reproduce(
                input_id,
                lsp_input,
                &self.target_executable,
                &self.target_args,
                false,
            )
            .with_context(|| format!("Reproducing crash for {}", input_file.display()))
        };
        let reproduction_infos: Vec<_> = if self.no_parallel {
            input_files
                .map(reproduce_one)
                .filter_map(Result::unwrap)
                .collect()
        } else {
            input_files
                .par_bridge()
                .map(reproduce_one)
                .filter_map(Result::unwrap)
                .collect()
        };

        let mut output_file = File::create(&self.output_file).context("Creating output file")?;
        serde_json::to_writer(&mut output_file, &reproduction_infos)
            .context("Writing output file")?;
        Ok(())
    }
}
