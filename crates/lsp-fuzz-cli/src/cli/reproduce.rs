use std::path::PathBuf;

use super::GlobalOptions;

/// Reproduces crashes found during fuzzing.
#[derive(Debug, clap::Parser)]
pub(super) struct ReproduceCommand {
    /// The path to the input file that caused the crash.
    #[clap(long, short)]
    input_file: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,
}

impl ReproduceCommand {
    pub fn run(self, _global_opttions: GlobalOptions) -> anyhow::Result<()> {
        // 1. Run the target with ASAN enabled.
        // 2. Feed the requests one by one to the target.
        // 3. If the target crashes, save the request that caused the crash.
        // 4. Grab the stack trace produced by ASAN.
        Ok(())
    }
}
