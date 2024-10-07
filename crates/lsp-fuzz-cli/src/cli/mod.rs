mod fuzz;

use anyhow::Context;
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
            Command::Fuzz(cli) => cli.run(self.global_options),
        }
    }
}

#[derive(Debug, clap::Parser)]
struct GlobalOptions {
    #[clap(long, default_value = "info")]
    default_log_level: LevelFilter,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Fuzz(fuzz::Cli),
}

fn setup_logger(global_opts: &GlobalOptions) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer().with_timer(fmt::time::LocalTime::rfc_3339()))
        .with(
            EnvFilter::builder()
                .with_default_directive(global_opts.default_log_level.into())
                .from_env()
                .context("Constructing log filter from env.")?,
        )
        .init();

    Ok(())
}
