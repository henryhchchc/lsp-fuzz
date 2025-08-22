mod baseline;
mod corpus_coverage;
mod export;
mod fuzz;
mod mine_grammar_fragments;
mod reproduce;

use std::{cmp::max, collections::HashMap, str::FromStr};

use anyhow::{Context, bail};
use baseline::{binary::BinaryBaseline, nautilus::NautilusBaseline};
use corpus_coverage::CatInput;
use export::ExportCommand;
use fuzz::FuzzCommand;
use libafl::inputs::{BytesInput, NautilusInput};
use lsp_fuzz::{baseline::BaselineInput, lsp_input::LspInput};
use mine_grammar_fragments::MineGrammarFragments;
use reproduce::{
    reproduce_all::ReproduceAll, reproduce_baseline::ReproduceBaseline, reproduce_one::ReproduceOne,
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::baseline::two_dim::TwoDimyBaseline;

#[derive(Debug, clap::Parser)]
#[command(version, about, styles = clap::builder::Styles::styled())]
pub struct Cli {
    #[clap(flatten)]
    global_options: GlobalOptions,

    #[command(subcommand)]
    command: Command,
}
impl Cli {
    pub(super) fn run(self) -> anyhow::Result<()> {
        self.global_options
            .setup_rayon()
            .context("Setting up rayon")?;
        setup_logger(&self.global_options).context("Setting up logger")?;
        match self.command {
            Command::Fuzz(cmd) => cmd.run(self.global_options),
            Command::BaselineNautilus(cmd) => cmd.run(self.global_options),
            Command::BaselineBinary(cmd) => cmd.run(self.global_options),
            Command::Baseline2D(cmd) => cmd.run(self.global_options),
            Command::Export(cmd) => cmd.run(self.global_options),
            Command::MineGrammarFragments(cmd) => cmd.run(self.global_options),
            Command::ReproduceOne(cmd) => cmd.run(self.global_options),
            Command::ReproduceAll(cmd) => cmd.run(self.global_options),
            Command::CatInput(cmd) => cmd.run(self.global_options),
            Command::CatBinInput(cmd) => cmd.run(self.global_options),
            Command::CatGramInput(cmd) => cmd.run(self.global_options),
            Command::ReproduceBaseline(cmd) => cmd.run(self.global_options),
        }
    }
}

#[derive(Debug, clap::Parser)]
struct GlobalOptions {
    #[clap(long, default_value = "info")]
    default_log_level: LevelFilter,

    #[clap(long)]
    random_seed: Option<u64>,

    #[clap(long)]
    parallel_workers: Option<usize>,
}

impl GlobalOptions {
    pub fn setup_rayon(&self) -> Result<(), rayon::ThreadPoolBuildError> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(self.parallel_workers())
            .build_global()
    }

    pub fn parallel_workers(&self) -> usize {
        self.parallel_workers
            .unwrap_or_else(|| max(1, num_cpus::get() / 2))
    }
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Fuzz(Box<FuzzCommand>),
    BaselineNautilus(Box<NautilusBaseline>),
    BaselineBinary(Box<BinaryBaseline>),
    Baseline2D(Box<TwoDimyBaseline>),
    MineGrammarFragments(MineGrammarFragments),
    Export(ExportCommand),
    ReproduceAll(ReproduceAll),
    ReproduceOne(ReproduceOne),
    ReproduceBaseline(ReproduceBaseline),
    CatInput(CatInput<LspInput>),
    CatBinInput(CatInput<BaselineInput<BytesInput>>),
    CatGramInput(CatInput<BaselineInput<NautilusInput>>),
}

fn setup_logger(global_opts: &GlobalOptions) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer().with_timer(fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_owned())))
        .with(
            EnvFilter::builder()
                .with_default_directive(global_opts.default_log_level.into())
                .from_env()
                .context("Constructing log filter from env.")?,
        )
        .init();

    Ok(())
}

pub fn parse_hash_map<K, V>(s: &str) -> Result<HashMap<K, V>, anyhow::Error>
where
    K: FromStr + std::hash::Hash + Eq,
    V: FromStr,
    <K as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    <V as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    let mut result = HashMap::new();
    if s.is_empty() {
        return Ok(result);
    }
    for pair in s.split(',') {
        let (key, value) = pair.split_once('=').context("Splitting key and value")?;
        let key = key.parse().context("Parsing key")?;
        let value = value.parse().context("Parsing value")?;
        result.insert(key, value);
    }
    Ok(result)
}

pub fn parse_size(s: &str) -> Result<usize, anyhow::Error> {
    if s.chars().last().is_some_and(|it| it.is_alphabetic()) {
        let (size, unit) = s.split_at(s.len() - 1);
        let muiltiplier = match unit.to_uppercase().as_str() {
            "B" => 1 << 0,
            "K" => 1 << 10,
            "M" => 1 << 20,
            "G" => 1 << 30,
            "T" => 1 << 40,
            _ => bail!("Invalid unit"),
        };
        let base_size: usize = size.parse()?;
        Ok(base_size * muiltiplier)
    } else {
        Ok(s.parse()?)
    }
}
