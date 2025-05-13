use std::{
    ffi::CStr,
    fs::File,
    io::{self, BufReader, ErrorKind, Write},
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::Duration,
};

use anyhow::Context;
use libafl::{
    generators::NautilusContext,
    inputs::{
        BytesInput, Input, InputToBytes, NautilusBytesConverter, NautilusInput, NopBytesConverter,
    },
};
use lsp_fuzz::baseline::{BaselineByteConverter, BaselineInput};
use nix::libc;
use rayon::iter::{ParallelBridge, ParallelIterator};
use tracing::{info, warn};

use super::ReproductionInfo;
use crate::cli::{
    GlobalOptions,
    reproduce::{ASAN_LOG_FN, asan_options, parse_asan_log},
};

/// Reproduces crashes found during fuzzing (for a directory containing the inputs).
#[derive(Debug, clap::Parser)]
pub struct ReproduceBaseline {
    /// The path to the directory containing the fuzz solutions.
    #[clap(long, short)]
    solution_dir: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_args: Vec<String>,

    /// The path to the output file.
    #[clap(long, short)]
    output_file: PathBuf,

    #[clap(long)]
    no_parallel: bool,

    #[clap(long, value_enum)]
    baseline_mode: BaselineMode,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum BaselineMode {
    Binary,
    Grammar,
}

impl ReproduceBaseline {
    pub fn run(self, _global_options: GlobalOptions) -> anyhow::Result<()> {
        info!(?self);
        let input_files = self
            .solution_dir
            .read_dir()
            .context("Reading solution directory")?
            .map(Result::unwrap)
            .filter(|it| {
                it.metadata().is_ok_and(|it| it.is_file())
                    && it.file_name().to_string_lossy().starts_with("input_")
            })
            .map(|it| it.path());

        static NAUTILUS_CONTEXT: OnceLock<NautilusContext> = OnceLock::new();
        let nautilus_context = NAUTILUS_CONTEXT.get_or_init(|| {
            let mut nautilus_ctx = NautilusContext {
                ctx: lsp_fuzz::lsp::metamodel::get_nautilus_context(),
            };
            nautilus_ctx.ctx.initialize(65535);
            nautilus_ctx
        });

        let reproduce_one = |input_file: PathBuf| {
            let input_id = input_file
                .file_name()
                .expect("We have checked that it is a file")
                .to_str()
                .context("The file name is not valid UTF-8")?
                .to_owned();
            let input_bytes = match self.baseline_mode {
                BaselineMode::Binary => {
                    let input = BaselineInput::<BytesInput>::from_file(&input_file)
                        .with_context(|| format!("Loading input file: {}", input_file.display()))?;
                    let mut bytes_input_converter =
                        BaselineByteConverter::new(NopBytesConverter::default());
                    bytes_input_converter.to_bytes(&input).to_vec()
                }
                BaselineMode::Grammar => {
                    let mut gram_input_converter =
                        BaselineByteConverter::new(NautilusBytesConverter::new(nautilus_context));
                    let input = BaselineInput::<NautilusInput>::from_file(&input_file)
                        .with_context(|| format!("Loading input file: {}", input_file.display()))?;
                    gram_input_converter.to_bytes(&input).to_vec()
                }
            };
            info!("Reproducing crash for input {}", input_id);
            reproduce_baseline(
                input_id,
                input_bytes,
                &self.target_executable,
                &self.target_args,
                false,
            )
            .with_context(|| format!("Reproducing crash for {}", input_file.display()))
        };
        let reproduction_infos: Vec<_> = if self.no_parallel {
            input_files
                .map(reproduce_one)
                .filter_map(Result::unwrap)
                .collect()
        } else {
            input_files
                .par_bridge()
                .map(reproduce_one)
                .filter_map(Result::unwrap)
                .collect()
        };

        let mut output_file = File::create(&self.output_file).context("Creating output file")?;
        serde_json::to_writer(&mut output_file, &reproduction_infos)
            .context("Writing output file")?;
        Ok(())
    }
}

fn reproduce_baseline(
    input_id: String,
    input: Vec<u8>,
    target_executable: &Path,
    target_args: &[String],
    show_stderr: bool,
) -> Result<Option<ReproductionInfo>, anyhow::Error> {
    let temp_directory = tempfile::tempdir().context("Creating temporary working directory")?;
    let workspace_dir = temp_directory.path();
    let asan_log_file_prefix = workspace_dir.join(ASAN_LOG_FN);
    let asan_options_env = asan_options(&asan_log_file_prefix).join(":");
    let mut target = Command::new(target_executable);
    target
        .args(target_args)
        .env("ASAN_OPTIONS", asan_options_env)
        .current_dir(workspace_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(if show_stderr {
            Stdio::inherit()
        } else {
            Stdio::null()
        });
    let mut child = target.spawn().context("Starting target process")?;
    let mut stdin = child.stdin.take().expect("We set it to pipe");
    match stdin.write_all(&input) {
        Ok(_) => {}
        Err(err) if err.kind() == ErrorKind::BrokenPipe => {}
        err => err?,
    }
    if child.try_wait().context("Trying to wait target")?.is_none() {
        std::thread::sleep(Duration::from_secs(15));
        if child.try_wait().context("Trying to wait target")?.is_none() {
            child.kill().context("Killing child")?;
        }
    }
    let status = child.wait().context("Waiting for target to exit")?;
    info!("Target exited with status: {:?}", status);

    if status.success() {
        info!("Target exited successfully");
        return Ok(None);
    } else if let Some(signal) = status.signal() {
        let signal_name = unsafe { CStr::from_ptr(libc::strsignal(signal)) };
        let signal_name = signal_name.to_string_lossy();
        info!("Target exited with signal: {}", signal_name);
    }

    let pid = child.id();
    let asan_log_file_path = asan_log_file_prefix.with_extension(child.id().to_string());
    let mut asan_log = match File::open(&asan_log_file_path) {
        Ok(file) => BufReader::new(file),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            warn!("ASAN log file not found");
            return Ok(None);
        }
        Err(e) => {
            return Err(e).context("Opening ASAN log file");
        }
    };
    let (asan_summary, classification, stack_trace) =
        parse_asan_log(&mut asan_log, pid).context("Parsing ASAN logs")?;
    info!(?classification);
    info!(location = ?stack_trace.first());
    Ok(Some(ReproductionInfo {
        input_id,
        input: None,
        crashing_request_idx: None,
        crashing_request: None,
        asan_summary,
        asan_classification: classification,
        stack_trace,
    }))
}
