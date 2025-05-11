use std::{
    fmt::Debug,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Context;
use derive_new::new as New;
use itertools::Itertools;
use libafl::{
    corpus::CorpusId,
    generators::NautilusContext,
    inputs::{
        BytesInput, Input, InputToBytes, NautilusBytesConverter, NautilusInput, NopBytesConverter,
    },
};
use lsp_fuzz::{
    baseline::{BaselineByteConverter, BaselineInput},
    coverage::CoverageDataGenerator,
    lsp_input::{LspInput, LspInputBytesConverter},
};
use rayon::prelude::*;
use tempfile::TempDir;
use tracing::info;

use crate::{cli::GlobalOptions, fuzzing::FuzzerStateDir};

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct CorpusCoverage<I> {
    /// Directory containing the fuzzer states.
    #[clap(long)]
    state: FuzzerStateDir,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_args: Vec<String>,

    #[clap(skip)]
    _input: PhantomData<I>,
}

impl<I> CorpusCoverage<I>
where
    I: Input + Clone + Send,
    ExperimentalCovByteGen: CovInputBytesGenerator<I>,
{
    pub fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        info!("Loading corpus");
        let covereage_inputs: Vec<CoverageInput<I>> =
            load_corpus(&self.state.corpus_dir()).context("Loading corpus")?;
        info!("Generating lcov reports");
        let coverage_data_generator =
            CoverageDataGenerator::new(self.target_executable, self.target_args);
        let input_by_gen_time = covereage_inputs
            .into_iter()
            .into_group_map_by(|it| it.time / 60);
        for i in 0..(24 * 60) {
            let inputs = input_by_gen_time
                .get(&i)
                .map(|it| it.len())
                .unwrap_or_default();
            info!("Minute: {i}, inputs: {inputs}");
        }

        let temp_dir = TempDir::new().context("Creating temp_dir")?;
        let input_bytes_conv = ExperimentalCovByteGen::new(temp_dir);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CoverageInput<I> {
    id: CorpusId,
    time: u64,
    exec: u64,
    content: I,
}

pub trait CovInputBytesGenerator<I> {
    fn generate_bytes(&self, input: &CoverageInput<I>) -> Vec<u8>;
}

#[derive(Debug, New)]
pub struct ExperimentalCovByteGen {
    temp_dir: TempDir,
}

impl CovInputBytesGenerator<LspInput> for ExperimentalCovByteGen {
    fn generate_bytes(&self, input: &CoverageInput<LspInput>) -> Vec<u8> {
        let mut converter = LspInputBytesConverter::new(self.temp_dir.path().to_owned());
        converter.to_bytes(&input.content).to_vec()
    }
}

impl CovInputBytesGenerator<BaselineInput<BytesInput>> for ExperimentalCovByteGen {
    fn generate_bytes(&self, input: &CoverageInput<BaselineInput<BytesInput>>) -> Vec<u8> {
        let mut converter = BaselineByteConverter::new(NopBytesConverter::default());
        converter.to_bytes(&input.content).to_vec()
    }
}

impl CovInputBytesGenerator<BaselineInput<NautilusInput>> for ExperimentalCovByteGen {
    fn generate_bytes(&self, input: &CoverageInput<BaselineInput<NautilusInput>>) -> Vec<u8> {
        static NAUTILUS_CONTEXT: OnceLock<NautilusContext> = OnceLock::new();
        let nautilus_context = NAUTILUS_CONTEXT.get_or_init(|| {
            let mut nautilus_ctx = NautilusContext {
                ctx: lsp_fuzz::lsp::metamodel::get_nautilus_context(),
            };
            nautilus_ctx.ctx.initialize(65535);
            nautilus_ctx
        });
        let mut converter =
            BaselineByteConverter::new(NautilusBytesConverter::new(nautilus_context));
        converter.to_bytes(&input.content).to_vec()
    }
}

fn inter_metadata(file_name: &str) -> Option<(CorpusId, u64, u64)> {
    // "id_{id}_time_{time}_exec_{exec}"
    let strip_id = file_name.strip_prefix("id_")?;
    let (id, remaining) = strip_id.split_once("_time_")?;
    let (time, exec) = remaining.split_once("_exec_")?;
    let id = id.parse().ok()?;
    let time = time.parse().ok()?;
    let exec = exec.parse().ok()?;
    Some((CorpusId(id), time, exec))
}

fn load_cov_input<I>(corpus_dir: &Path, input_file_name: String) -> anyhow::Result<CoverageInput<I>>
where
    I: Input,
{
    let input_path = corpus_dir.join(&input_file_name);
    let input = I::from_file(input_path).context("Loading input")?;
    let (id, time, exec) = inter_metadata(&input_file_name).context("Inter metadata")?;
    Ok(CoverageInput {
        id,
        content: input,
        time,
        exec,
    })
}

fn load_corpus<I>(corpus_dir: &Path) -> anyhow::Result<Vec<CoverageInput<I>>>
where
    I: Clone + Input + Send,
{
    corpus_dir
        .read_dir()
        .context("Reading corpus directory")?
        .map(Result::unwrap)
        .filter(|it| {
            it.metadata().is_ok_and(|it| it.is_file())
                && it.file_name().to_string_lossy().starts_with("id_")
        })
        .map(|it| {
            it.file_name()
                .into_string()
                .expect("File name should be valid UTF-8")
        })
        .par_bridge()
        .map(|it| load_cov_input(corpus_dir, it))
        .try_fold_with(Vec::default(), |mut acc, item| -> anyhow::Result<_> {
            let item = item?;
            acc.push(item);
            Ok(acc)
        })
        .try_reduce_with(|mut lhs, rhs| {
            lhs.extend(rhs);
            Ok(lhs)
        })
        .unwrap_or_else(|| Ok(Vec::default()))
}
