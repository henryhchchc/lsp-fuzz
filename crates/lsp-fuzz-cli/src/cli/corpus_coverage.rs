use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::Context;
use libafl::inputs::{BytesInput, Input, NautilusInput};
use libafl_bolts::serdeany::SerdeAnyMap;
use lsp_fuzz::{baseline::BaselineInput, corpus::GeneratedStats, lsp_input::LspInput};

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

    #[clap(long, value_enum)]
    corpus_type: CorpusType,
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
            })
            .map(|it| self.corpus_type.load_input(&self.state.solution_dir(), it));

        Ok(())
    }
}

#[derive(Debug)]
pub struct CoverageInput {
    id: String,
    input: CoverageInputRepr,
    stats: GeneratedStats,
}

impl CoverageInput {
    fn generate_lcov(&self, lcov_dir: &Path) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum CoverageInputRepr {
    LspFuzz(LspInput),
    BaselineBin(BaselineInput<BytesInput>),
    BaselineGram(BaselineInput<NautilusInput>),
}

impl CoverageInputRepr {
    fn generate_bytes(&self, workspace_dir: &Path) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CorpusType {
    LspFuzz,
    BaselineBin,
    BaselineGram,
}

impl CorpusType {
    fn load_input(
        &self,
        corpus_dir: &Path,
        input_file_name: String,
    ) -> anyhow::Result<CoverageInput> {
        let input_path = corpus_dir.join(&input_file_name);
        let input = match self {
            CorpusType::LspFuzz => {
                let input = LspInput::from_file(input_path).context("Loading LSP input")?;
                CoverageInputRepr::LspFuzz(input)
            }
            CorpusType::BaselineBin => {
                let input = BaselineInput::from_file(input_path)
                    .context("Loading baseline binary input")?;
                CoverageInputRepr::BaselineBin(input)
            }
            CorpusType::BaselineGram => {
                let input = BaselineInput::from_file(input_path)
                    .context("Loading baseline grammar input")?;
                CoverageInputRepr::BaselineGram(input)
            }
        };
        let metadata_reader = File::open(corpus_dir.join(format!(".{input_file_name}.metadata")))
            .context("Opening metadata file")?;
        let metadata_reader = BufReader::new(metadata_reader);
        let mut metadata_map: SerdeAnyMap =
            serde_json::from_reader(metadata_reader).context("Deserializing metadata")?;
        let stats: GeneratedStats = *metadata_map.remove().context("Getting GeneratedStats")?;

        Ok(CoverageInput {
            id: input_file_name,
            input,
            stats,
        })
    }
}
