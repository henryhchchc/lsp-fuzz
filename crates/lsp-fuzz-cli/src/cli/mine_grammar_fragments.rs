use super::GlobalOptions;

#[derive(Debug, clap::Parser)]
pub(super) struct MineGrammarFragments {}

impl MineGrammarFragments {
    pub(super) fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        todo!()
    }
}
