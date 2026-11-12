use std::{
    fs,
    io::{self, BufReader, Seek, Write},
    marker::PhantomData,
    os::fd::AsFd,
    sync::LazyLock,
};

use itertools::Itertools;
use libafl::{
    HasMetadata, HasToTargetBytesConverter,
    executors::{Executor, ExitKind, HasObservers},
    inputs::ToTargetBytesConverter,
    observers::{AsanBacktraceObserver, MapObserver, ObserversTuple},
    state::HasExecutions,
};
use libafl_bolts::{
    HasLen, Truncate,
    shmem::{ShMem, ShMemId},
    tuples::RefIndexable,
};
use nix::{sys::time::TimeSpec, unistd::Pid};
use tempfile::{NamedTempFile, TempDir};
use tracing::info;

use super::{
    fork_server::{self, FuzzInputSetup, NeoForkServer, NeoForkServerOptions},
    observers::Observers,
    responses::LspOutputObserver,
    target::FuzzTargetInfo,
    transport::FuzzInput,
};
use crate::{libafl_support::AflContext, utf8::UTF8Tokens};

static ASAN_LOG_DIR: LazyLock<TempDir> =
    LazyLock::new(|| TempDir::new().expect("Failed to create ASAN log directory"));

static ASAN_LOG_PATH: LazyLock<&str> = LazyLock::new(|| {
    ASAN_LOG_DIR
        .path()
        .to_str()
        .expect("Temp dir is expect to be valid UTF-8")
});

#[derive(Debug)]
pub struct FuzzExecutionConfig<'a, SHM, MO, OBS> {
    pub debug_child: bool,
    pub debug_afl: bool,
    pub fuzz_input: FuzzInput<SHM>,
    pub auto_tokens: Option<&'a mut UTF8Tokens>,
    pub coverage_shm_info: (ShMemId, usize),
    pub map_observer: MO,
    pub responses_observer: LspOutputObserver,
    pub asan_observer: Option<AsanBacktraceObserver>,
    pub other_observers: OBS,
}

#[derive(Debug)]
pub struct LspExecutor<State, MO, OBS, I, SHM> {
    fork_server: NeoForkServer,
    crash_exit_code: Option<i8>,
    timeout: TimeSpec,
    fuzz_input: FuzzInput<SHM>,
    output_capture_file: NamedTempFile,
    observers: Observers<MO, OBS>,
    _state: PhantomData<(State, I)>,
}

impl<State, OBS, MO, I, SHM> LspExecutor<State, MO, OBS, I, SHM>
where
    SHM: ShMem,
{
    /// Create and initialize a new LSP executor.
    ///
    /// # Errors
    ///
    /// Returns an error if the fork server cannot be initialized, the output capture file cannot
    /// be created, or the target's runtime options are incompatible with the configured observers
    /// or input transport.
    pub fn start<A>(
        target_info: FuzzTargetInfo,
        mut config: FuzzExecutionConfig<'_, SHM, MO, OBS>,
    ) -> Result<Self, libafl::Error>
    where
        MO: AsRef<A> + AsMut<A>,
        A: Truncate + HasLen + MapObserver,
    {
        let args = target_info.args.into_iter().map(Into::into).collect();

        let asan_options = [
            "detect_odr_violation=0",
            "abort_on_error=1",
            "symbolize=0",
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
        ];
        let asan_log_path = config
            .asan_observer
            .as_ref()
            .map(|_| format!("log_path={}", *ASAN_LOG_PATH));

        let asan_options = asan_options
            .into_iter()
            .chain(asan_log_path.as_deref())
            .join(":");

        let mut envs = vec![("ASAN_OPTIONS".into(), asan_options.into())];

        envs.extend(
            target_info
                .env
                .into_iter()
                .map(|(k, v)| (k.into(), v.into())),
        );

        let output_capture_file =
            NamedTempFile::new().afl_context("Creating output capture file")?;

        let opts = NeoForkServerOptions {
            target: target_info.path.as_os_str().to_owned(),
            args,
            envs,
            input_setup: FuzzInputSetup::from(&config.fuzz_input),
            memlimit: 0,
            persistent_fuzzing: target_info.persistent_fuzzing,
            deferred: target_info.defer_fork_server,
            coverage_map_info: config.coverage_shm_info,
            afl_debug: config.debug_afl,
            debug_output: config.debug_child,
            kill_signal: target_info.kill_signal,
            stdout_capture_fd: output_capture_file.as_fd(),
        };
        let mut fork_server = fork_server::NeoForkServer::new(opts)?;

        let options = fork_server
            .initialize()
            .afl_context("Initializing fork server")?;

        if let Some(fsrv_map_size) = options.map_size {
            match config.map_observer.as_ref().len() {
                map_size if map_size > fsrv_map_size => {
                    config.map_observer.as_mut().truncate(fsrv_map_size);
                    info!(new_size = fsrv_map_size, "Coverage map truncated");
                }
                map_size if map_size < fsrv_map_size => {
                    Err(libafl::Error::illegal_argument(format!(
                        "The map size is too small. {fsrv_map_size} is required for the target."
                    )))?;
                }
                map_size if map_size == fsrv_map_size => {}
                _ => unreachable!("Guaranteed by the match statement above."),
            }
        }

        if matches!(config.fuzz_input, FuzzInput::SharedMemory(_) if !options.shmem_fuzz) {
            Err(libafl::Error::unknown(
                "Target requested sharedmem fuzzing, but you didn't prepare shmem",
            ))?;
        }

        if let (Some(auto_dict), Some(auto_dict_payload)) = (config.auto_tokens, options.autodict) {
            auto_dict.parse_auto_dict(auto_dict_payload);
        }

        let observers = Observers {
            map_observer: config.map_observer,
            responses_observer: config.responses_observer,
            asan_observer: config.asan_observer,
            extra: config.other_observers,
        };

        Ok(Self {
            fork_server,
            crash_exit_code: target_info.crash_exit_code,
            timeout: target_info.timeout,
            fuzz_input: config.fuzz_input,
            output_capture_file,
            observers,
            _state: PhantomData,
        })
    }

    fn clear_output_capture_file(&mut self) -> io::Result<()> {
        let output_capture_file = self.output_capture_file.as_file_mut();
        output_capture_file.rewind()?;
        output_capture_file.write_all(&[])?;
        output_capture_file.set_len(0)?;
        output_capture_file.flush()?;
        output_capture_file.sync_data()?;
        Ok(())
    }
}

impl<State, MO, OBS, I, SHM> HasObservers for LspExecutor<State, MO, OBS, I, SHM>
where
    OBS: ObserversTuple<I, State>,
{
    type Observers = Observers<MO, OBS>;

    fn observers(&self) -> RefIndexable<&Self::Observers, Self::Observers> {
        RefIndexable::from(&self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut Self::Observers, Self::Observers> {
        RefIndexable::from(&mut self.observers)
    }
}

impl<EM, I, Z, State, MO, OBS, SHM> Executor<EM, I, State, Z>
    for LspExecutor<State, MO, OBS, I, SHM>
where
    Observers<MO, OBS>: ObserversTuple<I, State>,
    State: HasExecutions + HasMetadata,
    SHM: ShMem,
    Z: HasToTargetBytesConverter,
    Z::Converter: ToTargetBytesConverter<I, State>,
{
    fn run_target(
        &mut self,
        fuzzer: &mut Z,
        state: &mut State,
        _mgr: &mut EM,
        input: &I,
    ) -> Result<ExitKind, libafl::Error> {
        // Transfer input to the fork server
        let bytes = fuzzer
            .target_bytes_converter_mut()
            .convert_to_target_bytes(state, input);
        let input_bytes = bytes;
        self.fuzz_input.send(&input_bytes)?;

        self.clear_output_capture_file()
            .afl_context("Clearing output capture file")?;

        let (child_pid, status) = self.fork_server.run_child(&self.timeout)?;

        let exit_kind = if let Some(status) = status {
            let exitcode_is_crash = self
                .crash_exit_code
                .filter(|_| libc::WIFEXITED(status))
                .is_some_and(|it| libc::WEXITSTATUS(status) == i32::from(it));
            if libc::WIFSIGNALED(status) || exitcode_is_crash {
                ExitKind::Crash
            } else {
                ExitKind::Ok
            }
        } else {
            ExitKind::Timeout
        };
        if exit_kind == ExitKind::Ok {
            self.output_capture_file
                .rewind()
                .afl_context("Rewinding output capture file")?;
            let output_reader = BufReader::new(&mut self.output_capture_file);
            self.observers
                .responses_observer
                .capture_stdout_content(output_reader)
                .afl_context("Capturing target output")?;
        }
        if exit_kind == ExitKind::Crash
            && let Some(ref mut asan_observer) = self.observers.asan_observer
            && let Some(ref asan_log_content) = read_asan_log(child_pid)?
        {
            let log_content = String::from_utf8_lossy(asan_log_content);
            asan_observer.parse_asan_output(log_content.as_ref());
        }

        *state.executions_mut() += 1;
        Ok(exit_kind)
    }
}

fn read_asan_log(child_pid: Pid) -> Result<Option<Vec<u8>>, libafl::Error> {
    let asan_log_file = format!("{}.{child_pid}", *ASAN_LOG_PATH);
    let log = if fs::exists(&asan_log_file)? {
        let asan_log = fs::read(&asan_log_file).afl_context("Reading ASAN log file")?;
        fs::remove_file(asan_log_file).afl_context("Fail to cleanup ASAN log file")?;
        Some(asan_log)
    } else {
        None
    };
    Ok(log)
}
