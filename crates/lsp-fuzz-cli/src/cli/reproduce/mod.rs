use std::{
    borrow::Cow,
    ffi::CStr,
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Write},
    os::{fd::AsFd, unix::process::ExitStatusExt},
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::Context;
use itertools::Itertools;
use libcasr::{
    asan::{AsanContext, AsanStacktrace},
    execution_class::ExecutionClass,
    severity::Severity,
    stacktrace::ParseStacktrace,
};
use lsp_fuzz::{lsp::json_rpc::JsonRPCMessage, lsp_input::LspInput};
use nix::libc;
use serde::Serialize;
use tracing::{info, warn};

pub mod reproduce_all;
pub mod reproduce_one;

fn json_rpc_messages<'a>(
    lsp_input: &'a LspInput,
    workspace_url: &'a str,
) -> impl Iterator<Item = JsonRPCMessage> + use<'a> {
    let mut msg_id = 0;
    lsp_input
        .message_sequence()
        .map(move |msg| msg.into_json_rpc(&mut msg_id, Some(workspace_url)))
}

fn find_crashing_request(
    input: &LspInput,
    workspace_url: &str,
    child: &mut Child,
) -> Result<Option<JsonRPCMessage>, anyhow::Error> {
    let mut target_stdin = child
        .stdin
        .take()
        .context("Child should have its stdin piped")?;
    let target_stdout = child
        .stdout
        .take()
        .context("Child should have its stdout piped")?;
    let mut target_stdout = BufReader::new(target_stdout);
    let mut crashing_request = None;
    for jsonrpc in json_rpc_messages(input, workspace_url) {
        info!(
            id = ?jsonrpc.id(),
            method = ?jsonrpc.method(),
            "Sending message to target"
        );
        match target_stdin.write_all(&jsonrpc.to_lsp_payload()) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {}
            Err(e) => Err(e).context("Sending message to target")?,
        }
        if let JsonRPCMessage::Request { id: request_id, .. } = &jsonrpc {
            loop {
                let mut fdset = nix::sys::select::FdSet::new();
                fdset.insert(target_stdout.get_ref().as_fd());
                let timeout = &Duration::from_secs(30).into();
                match nix::sys::select::pselect(None, &mut fdset, None, None, Some(timeout), None) {
                    Ok(1) => {}
                    Ok(0) => {
                        warn!("Timeout waiting for target to respond");
                        child.kill().context("Killing target")?;
                        break;
                    }
                    Ok(_) => unreachable!("we passed in only one fd"),
                    Err(e) => return Err(e).context("Error in pselect"),
                };

                match JsonRPCMessage::read_lsp_payload(&mut target_stdout) {
                    Ok(JsonRPCMessage::Response { id, .. }) => {
                        info!(?id, "Received an response from target");
                        if id.is_some_and(|it| it == *request_id) {
                            break;
                        }
                    }
                    Ok(JsonRPCMessage::Notification { .. }) => {
                        info!("Received a notification from target");
                    }
                    Ok(JsonRPCMessage::Request { method, .. }) => {
                        info!(%method, "Received a request from target");
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                        break;
                    }
                    Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                        warn!(error = ?e, "Invalid data read from the target");
                        break;
                    }
                    Err(e) => {
                        return Err(e).context("Reading from target");
                    }
                }
            }
        }
        if let Some(status) = child.try_wait().context("Waiting child")? {
            if !status.success() {
                crashing_request = Some(jsonrpc);
            }
            break;
        }
    }
    Ok(crashing_request)
}

const ASAN_LOG_FN: &str = "lsp-fuzz-asan";

#[tracing::instrument(skip(input, target_executable, target_args))]
fn repdoruce(
    input_id: String,
    input: LspInput,
    target_executable: &Path,
    target_args: &[String],
) -> Result<Option<ReproductionInfo>, anyhow::Error> {
    let temp_directory = tempfile::tempdir().context("Creating temporary working directory")?;
    let workspace_dir = temp_directory.path();
    let asan_log_file_prefix = workspace_dir.join(ASAN_LOG_FN);
    let asan_options_env = asan_options(&asan_log_file_prefix).join(":");
    input
        .setup_source_dir(workspace_dir)
        .context("Setting up workspace_dir")?;
    let mut target = Command::new(target_executable);
    target
        .args(target_args)
        .env("ASAN_OPTIONS", asan_options_env)
        .current_dir(workspace_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = target.spawn().context("Starting target process")?;
    let workspace_url = format!(
        "file://{}/",
        workspace_dir
            .to_str()
            .expect("The workspace_dir is not valid UTF-8")
    );
    let crashing_request = find_crashing_request(&input, &workspace_url, &mut child)?;
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

    let crashing_request = crashing_request.expect("The target should have crashed");

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
        input,
        crashing_request,
        asan_summary,
        asan_classification: classification,
        stack_trace,
    }))
}

fn parse_asan_log<R: Read>(
    asan_log: &mut R,
    pid: u32,
) -> Result<(String, Option<ExecutionClass>, Vec<StacktraceEntry>), anyhow::Error> {
    let mut log_content = String::new();
    asan_log
        .read_to_string(&mut log_content)
        .context("Reading ASAN log")?;
    info!(%log_content);
    let pid_prefix = format!("=={}==", pid);
    let asan_summary = log_content
        .lines()
        .skip(1)
        .filter_map(|it| it.strip_prefix(&pid_prefix))
        .join("\n");
    info!(asan_summary);
    let lines = log_content.lines().map(ToOwned::to_owned).collect();
    let classification = AsanContext(lines).severity().ok();
    let stack_trace =
        AsanStacktrace::extract_stacktrace(&log_content).context("Extracting stack trace")?;
    let stack_trace: Vec<_> = AsanStacktrace::parse_stacktrace(&stack_trace)
        .context("Parsing stack trace")?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((asan_summary, classification, stack_trace))
}

fn asan_options(asan_log_file: &Path) -> Vec<Cow<'_, str>> {
    let asan_log_file = asan_log_file
        .to_str()
        .expect("The temp path is not valid UTF-8");
    let log_config = format!("log_path={}", asan_log_file);
    [
        "detect_odr_violation=0",
        "abort_on_error=1",
        "symbolize=1",
        "allocator_may_return_null=1",
        "handle_segv=1",
        "handle_sigbus=1",
        "handle_sigfpe=1",
        "handle_sigill=1",
        "handle_abort=2", // Some targets may have their own abort handler
        "detect_stack_use_after_return=1",
        "check_initialization_order=0",
        "detect_leaks=1",
        "malloc_context_size=0",
    ]
    .into_iter()
    .map(Cow::Borrowed)
    .chain(std::iter::once(Cow::Owned(log_config)))
    .collect()
}

#[derive(Debug, Serialize)]
pub struct ReproductionInfo {
    pub input_id: String,
    pub input: LspInput,
    pub crashing_request: JsonRPCMessage,
    pub asan_summary: String,
    pub asan_classification: Option<ExecutionClass>,
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
