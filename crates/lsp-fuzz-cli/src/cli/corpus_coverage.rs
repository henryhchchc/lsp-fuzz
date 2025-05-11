use std::{
    fmt::Debug,
    fs,
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

use crate::{
    cli::GlobalOptions,
    fuzzing::{FuzzerStateDir, common::ParTryCollect},
};

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
    I: Input + Send + Sync,
    ExperimentalCovByteGen: CovInputBytesGenerator<I>,
{
    pub fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        info!("Loading corpus");
        let covereage_inputs: Vec<CoverageInput<I>> =
            load_corpus(&self.state.corpus_dir()).context("Loading corpus")?;
        fs::create_dir_all(self.state.coverage_dir()).context("Creating coverage dir")?;
        info!(
            "Generating coverage reports for {}",
            self.target_executable.display()
        );
        let coverage_data_generator =
            CoverageDataGenerator::new(self.target_executable, self.target_args);
        let temp_dir = TempDir::new().context("Creating temp_dir")?;
        let input_bytes_conv = ExperimentalCovByteGen::new(temp_dir);
        let input_by_gen_time = covereage_inputs
            .into_iter()
            .into_group_map_by(|it| it.time / 60);
        (0..(24 * 60))
            .into_par_iter()
            .map(|minute| -> anyhow::Result<_> {
                if let Some(inputs) = input_by_gen_time.get(&minute) {
                    info!("Minute: {minute}, inputs: {}", inputs.len());
                    let chunks: Vec<_> = inputs
                        .iter()
                        .chunks(10)
                        .into_iter()
                        .map(|it| it.collect_vec())
                        .collect();
                    let mut existing_data = None;
                    for chunk in chunks {
                        let cov_raw_data_dir =
                            TempDir::new().context("Crateing raw data tempdir")?;
                        let raw_data_files: Vec<_> = chunk
                            .into_par_iter()
                            .map(|it| -> anyhow::Result<_> {
                                let input_bytes = input_bytes_conv.generate_bytes(it);
                                let llvm_profile_raw =
                                    cov_raw_data_dir.path().join(it.raw_prof_data_file_name());
                                coverage_data_generator.run_target_with_coverage(
                                    &input_bytes,
                                    llvm_profile_raw.as_os_str(),
                                )?;
                                Ok(llvm_profile_raw)
                            })
                            .try_collect_par()
                            .context("Running target")?;
                        let cov_data = self.state.coverage_dir().join(format!("{minute}.profdata"));
                        coverage_data_generator
                            .merge_llvm_raw_prof_data(
                                raw_data_files.into_iter().chain(existing_data),
                                &cov_data,
                            )
                            .context("Merging coverage data")?;
                        existing_data = Some(cov_data);
                    }
                }
                Ok(())
            })
            .try_collect_par::<Vec<_>>()?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct CoverageInput<I> {
    id: CorpusId,
    time: u64,
    #[allow(unused, reason = "For completeness")]
    exec: u64,
    content: I,
}

impl<I> CoverageInput<I> {
    pub fn raw_prof_data_file_name(&self) -> String {
        format!("coverage.{}.profraw", self.id.0)
    }
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
        .try_collect_par()
}
