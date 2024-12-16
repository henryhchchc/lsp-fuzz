use super::GlobalOptions;

/// Fuzz a Language Server Protocol (LSP) server.
#[derive(Debug, clap::Parser)]
pub(super) struct MinimizeCommand {}

impl MinimizeCommand {
    pub(super) fn run(self, _global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        todo!()
    }
}
