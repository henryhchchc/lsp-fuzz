use std::ffi::CStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::Context;
use itertools::Itertools;
use libafl::inputs::Input;
use libcasr::asan::{AsanContext, AsanStacktrace};
use libcasr::execution_class::ExecutionClass;
use libcasr::severity::Severity;
use libcasr::stacktrace::ParseStacktrace;
use lsp_fuzz::lsp::ClientToServerMessage;
use lsp_fuzz::lsp_input::LspInput;
use nix::libc;
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::Serialize;
use tracing::info;

use super::GlobalOptions;

/// Reproduces crashes found during fuzzing.
#[derive(Debug, clap::Parser)]
pub(super) struct ReproduceCommand {
    /// The path to the directory containing the fuzz solutions.
    #[clap(long, short)]
    solution_dir: PathBuf,

    /// The path to the target executable.
    #[clap(long, short)]
    target_executable: PathBuf,

    /// The path to the output file.
    #[clap(long, short)]
    output_file: PathBuf,
}

const ASAN_LOG_FN: &str = "lsp-fuzz-asan";

impl ReproduceCommand {
    pub fn run(self, _global_opttions: GlobalOptions) -> anyhow::Result<()> {
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

        let reproduction_infos: Vec<_> = input_files
            .par_bridge()
            .map(|input_file| {
                let input_id = input_file
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let lsp_input = LspInput::from_file(&input_file).context("Loading input file")?;
                info!("Reproducing crash for input {}", input_id);
                repdoruce(input_id, lsp_input, &self.target_executable).context("Reproducing crash")
            })
            .filter_map(|it| it.unwrap())
            .collect();

        let mut output_file = File::create(&self.output_file).context("Creating output file")?;
        serde_json::to_writer(&mut output_file, &reproduction_infos)
            .context("Writing output file")?;
        Ok(())
    }
}

fn repdoruce(
    input_id: String,
    input: LspInput,
    target_program: &Path,
) -> Result<Option<ReproductionInfo>, anyhow::Error> {
    let temp_wd = tempfile::tempdir().context("Creating temporary working directory")?;
    let workspace_dir = temp_wd.path();
    let mut asan_options = vec![
        "detect_odr_violation=0",
        "abort_on_error=1",
        "symbolize=1",
        "allocator_may_return_null=1",
        "handle_segv=1",
        "handle_sigbus=1",
        "handle_sigfpe=1",
        "handle_sigill=1",
        "handle_abort=2", // Some targets may have their own abort handler
        "detect_stack_use_after_return=0",
        "check_initialization_order=0",
        "detect_leaks=0",
        "malloc_context_size=0",
    ];
    let asan_log_file_path = temp_wd.path().join(ASAN_LOG_FN);
    let log_config = &format!("log_path={}", asan_log_file_path.to_string_lossy(),);
    asan_options.push(log_config);
    let asan_options = asan_options.join(":");
    input
        .setup_source_dir(temp_wd.path())
        .context("Setting up workspace_dir")?;
    let mut target = Command::new(target_program);
    target
        .env("ASAN_OPTIONS", asan_options)
        .current_dir(temp_wd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = target.spawn().context("Starting target process")?;
    let mut msg_id = 0;
    let workspace_dir_str = workspace_dir.to_string_lossy();
    let workspace_dir_str = format!("file://{}/", workspace_dir_str);
    let mut target_stdin = child.stdin.take().expect("We set it to piped");
    let target_stdout = child.stdout.take().expect("We set it to piped");
    let mut target_stdout = BufReader::new(target_stdout);
    let mut crashing_request = None;
    for request in input.message_sequence(workspace_dir) {
        let expect_response = request.is_request();
        let jsonrpc = request
            .clone()
            .into_json_rpc(&mut msg_id, Some(workspace_dir_str.as_str()));
        info!(id = jsonrpc.id, method = %jsonrpc.method, "Sending message to target");
        target_stdin
            .write_all(&jsonrpc.to_lsp_payload())
            .context("Sending message to target")?;
        if expect_response {
            loop {
                if let Some(response) = grab_output(&mut target_stdout)? {
                    let value = serde_json::from_slice::<serde_json::Value>(&response)
                        .context("Parsing response")?;
                    if let Some(id) = value["id"].as_u64() {
                        info!(id, length = response.len(), "Get response");
                        break;
                    } else {
                        info!(length = response.len(), "Get notification");
                    }
                } else {
                    info!("No response");
                    break;
                }
            }
        }
        if (child.try_wait().context("Waiting child")?).is_some() {
            crashing_request = Some(request);
            break;
        }
    }
    let status = child.wait().context("Waiting for target to exit")?;
    info!("Target exited with status: {:?}", status);
    if status.success() {
        info!("Target exited successfully");
    } else if let Some(signal) = status.signal() {
        let signal_name = unsafe { CStr::from_ptr(libc::strsignal(signal)) };
        let signal_name = signal_name.to_string_lossy();
        info!("Target exited with signal: {}", signal_name);
    }
    let pid = child.id();
    let asan_log_file_path = asan_log_file_path.with_extension(child.id().to_string());
    Ok(if let Ok(asan_log_file) = File::open(&asan_log_file_path) {
        let mut asan_log = BufReader::new(asan_log_file);
        let mut log_content = String::new();
        asan_log
            .read_to_string(&mut log_content)
            .context("Reading ASAN log")?;
        let pid_prefix = format!("=={}==", pid);
        let summary = log_content
            .lines()
            .skip(1)
            .filter_map(|it| it.strip_prefix(&pid_prefix))
            .join("\n");
        info!(summary);
        let lines: Vec<_> = log_content.lines().map(str::to_string).collect();
        let classification = AsanContext(lines)
            .severity()
            .context("Getting ASAN severity")?;
        let stack_trace =
            AsanStacktrace::extract_stacktrace(&log_content).context("Extracting stack trace")?;
        let stack_trace: Vec<_> = AsanStacktrace::parse_stacktrace(&stack_trace)
            .context("Parsing stack trace")?
            .into_iter()
            .map(Into::into)
            .collect();
        info!(?classification);
        info!(location = ?stack_trace.first());
        Some(ReproductionInfo {
            input_id,
            input,
            crashing_request: crashing_request.unwrap(),
            asan_summary: summary,
            asan_classification: classification,
            stack_trace,
        })
    } else {
        None
    })
}

#[derive(Debug, Serialize)]
pub struct ReproductionInfo {
    pub input_id: String,
    pub input: LspInput,
    pub crashing_request: ClientToServerMessage,
    pub asan_summary: String,
    pub asan_classification: ExecutionClass,
    pub stack_trace: Vec<StacktraceEntry>,
}

#[derive(Debug, Serialize)]
pub struct StacktraceEntry {
    pub address: u64,
    pub function: String,
    pub module: String,
    pub offset: u64,
    pub debug: DebugInfo,
}

impl From<libcasr::stacktrace::StacktraceEntry> for StacktraceEntry {
    fn from(entry: libcasr::stacktrace::StacktraceEntry) -> Self {
        Self {
            address: entry.address,
            function: entry.function,
            module: entry.module,
            offset: entry.offset,
            debug: DebugInfo {
                file: entry.debug.file,
                line: entry.debug.line,
                column: entry.debug.column,
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DebugInfo {
    pub file: String,
    pub line: u64,
    pub column: u64,
}

fn grab_output(
    target_stdout: &mut BufReader<std::process::ChildStdout>,
) -> Result<Option<Vec<u8>>, anyhow::Error> {
    let mut size = None;
    let response_size = loop {
        let mut header = String::new();
        target_stdout
            .read_line(&mut header)
            .context("Reading response header")?;
        let header = header.trim_end_matches("\r\n");
        if header.is_empty() {
            break size;
        }
        let (name, value) = header.split_once(": ").context("Parsing header")?;
        if name == "Content-Length" {
            size = Some(value.parse::<usize>().context("Parsing Content-Length")?);
        }
    };
    Ok(if let Some(response_size) = response_size {
        let mut target_output = Vec::with_capacity(response_size);
        #[allow(clippy::uninit_vec)]
        unsafe {
            target_output.set_len(response_size);
        }
        target_stdout
            .read_exact(&mut target_output)
            .context("Reading response body")?;
        Some(target_output)
    } else {
        None
    })
}
