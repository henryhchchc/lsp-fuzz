mod fuzz;
mod mine_grammar_fragments;

use anyhow::Context;
use fuzz::FuzzCommand;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
        setup_logger(&self.global_options).context("Setting up logger")?;
        match self.command {
            Command::Fuzz(cmd) => cmd.run(self.global_options),
            Command::MineGrammarFragments(cmd) => cmd.run(self.global_options),
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
    pub fn parallel_workers(&self) -> usize {
        self.parallel_workers.unwrap_or_else(num_cpus::get)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Fuzz(FuzzCommand),
    MineGrammarFragments(mine_grammar_fragments::MineGrammarFragments),
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
