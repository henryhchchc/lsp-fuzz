use super::GlobalOptions;

/// Reproduces crashes found during fuzzing.
#[derive(Debug, clap::Parser)]
pub(super) struct ReproduceCommand {}

impl ReproduceCommand {
    pub fn run(self, _global_opttions: GlobalOptions) -> anyhow::Result<()> {
        Ok(())
    }
}
