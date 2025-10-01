use std::{
    io::{self, Write},
    marker::PhantomData,
    path::PathBuf,
    sync::OnceLock,
};

use anyhow::Context;
use derive_new::new as New;
use libafl::{
    generators::NautilusContext,
    inputs::{
        BytesInput, Input, NautilusBytesConverter, NautilusInput, NopToTargetBytes, ToTargetBytes,
    },
};
use lsp_fuzz::{
    baseline::{BaselineByteConverter, BaselineInput},
    lsp_input::{LspInput, LspInputBytesConverter},
};
use tempfile::TempDir;

use crate::cli::GlobalOptions;

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct CatInput<I> {
    #[clap(long)]
    input_path: PathBuf,

    #[clap(skip)]
    _input: PhantomData<I>,
}

impl<I> CatInput<I>
where
    I: Input + Send + Sync,
    ExperimentalCovByteGen: CovInputBytesGenerator<I>,
{
    pub fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        let input = I::from_file(&self.input_path).context("Reading input")?;
        let temp_dir = TempDir::new().context("Creating temp_dir")?;
        let input_bytes_conv = ExperimentalCovByteGen::new(temp_dir);

        let bytes = input_bytes_conv.generate_bytes(&input);

        io::stdout()
            .lock()
            .write_all(&bytes)
            .context("Writing to stdout")?;

        Ok(())
    }
}

pub trait CovInputBytesGenerator<I> {
    fn generate_bytes(&self, input: &I) -> Vec<u8>;
}

#[derive(Debug, New)]
pub struct ExperimentalCovByteGen {
    temp_dir: TempDir,
}

impl CovInputBytesGenerator<LspInput> for ExperimentalCovByteGen {
    fn generate_bytes(&self, input: &LspInput) -> Vec<u8> {
        let mut converter = LspInputBytesConverter::new(self.temp_dir.path().to_owned());
        converter.to_target_bytes(input).to_vec()
    }
}

impl CovInputBytesGenerator<BaselineInput<BytesInput>> for ExperimentalCovByteGen {
    fn generate_bytes(&self, input: &BaselineInput<BytesInput>) -> Vec<u8> {
        let mut converter = BaselineByteConverter::new(NopToTargetBytes::default());
        converter.to_target_bytes(input).to_vec()
    }
}

impl CovInputBytesGenerator<BaselineInput<NautilusInput>> for ExperimentalCovByteGen {
    fn generate_bytes(&self, input: &BaselineInput<NautilusInput>) -> Vec<u8> {
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
        converter.to_target_bytes(input).to_vec()
    }
}
