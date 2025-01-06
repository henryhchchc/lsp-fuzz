use std::{env::temp_dir, fs, marker::PhantomData, mem, path::PathBuf};

use fork_server::{FuzzInputSetup, NeoForkServer, NeoForkServerOptions};
use libafl::{
    executors::{Executor, ExitKind, HasObservers},
    inputs::UsesInput,
    mutators::Tokens,
    observers::{AsanBacktraceObserver, MapObserver, Observer, ObserversTuple},
    state::{HasExecutions, State, UsesState},
};
use libafl_bolts::{
    fs::InputFile,
    shmem::{ShMem, ShMemId},
    tuples::{Handle, MatchNameRef, Prepend, RefIndexable},
    AsSliceMut, Truncate,
};
use nix::{
    sys::{signal::Signal, time::TimeSpec},
    unistd::Pid,
};
use tracing::{info, warn};

use crate::{lsp_input::LspInput, utils::AflContext};

pub mod fork_server;

const ASAN_LOG_PATH: &str = "/tmp/asan";

/// Describes how the fuzz input is sent to the target.
#[derive(Debug)]
pub enum FuzzInput<SHM> {
    /// Send the input to the target via stdin.
    Stdin(InputFile),
    /// Send the input to the target via a file as an argument.
    File(InputFile),
    /// Send the input to the target via shared memory.
    SharedMemory(SHM),
}

impl<SHM: ShMem> FuzzInput<SHM> {
    pub fn send(&mut self, input_bytes: &[u8]) -> Result<(), libafl::Error> {
        match self {
            FuzzInput::Stdin(file) | FuzzInput::File(file) => file.write_buf(input_bytes),
            FuzzInput::SharedMemory(shmem) => Self::write_afl_shmem_input(shmem, input_bytes),
        }
    }

    const SHM_FUZZ_HEADER_SIZE: usize = mem::size_of::<u32>();

    fn write_afl_shmem_input(shmem: &mut SHM, input_bytes: &[u8]) -> Result<(), libafl::Error> {
        use core::sync::atomic::{compiler_fence, Ordering};

        compiler_fence(Ordering::SeqCst);

        if shmem.len() < input_bytes.len() + Self::SHM_FUZZ_HEADER_SIZE {
            Err(libafl::Error::unknown(
                "The shared memory is too small for the input.",
            ))?;
        }
        let input_size = u32::try_from(input_bytes.len())
            .afl_context("The length of input bytes cannot fit into u32")?;
        let input_size_encoded = input_size.to_ne_bytes();
        let shmem_slice = shmem.as_slice_mut();
        shmem_slice[..Self::SHM_FUZZ_HEADER_SIZE].copy_from_slice(&input_size_encoded);
        let input_body_range =
            Self::SHM_FUZZ_HEADER_SIZE..(Self::SHM_FUZZ_HEADER_SIZE + input_bytes.len());
        shmem_slice[input_body_range].copy_from_slice(input_bytes);

        compiler_fence(Ordering::SeqCst);

        Ok(())
    }
}

impl<SHM> Drop for FuzzInput<SHM> {
    fn drop(&mut self) {
        if let FuzzInput::File(file) | FuzzInput::Stdin(file) = self {
            if let Err(e) = fs::remove_file(&file.path) {
                warn!("Failed to delete file: {}", e);
            }
        }
    }
}

#[derive(Debug)]
pub struct FuzzTargetInfo {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub persistent_fuzzing: bool,
    pub defer_fork_server: bool,
    pub crash_exit_code: Option<i8>,
    pub timeout: TimeSpec,
    pub kill_signal: Signal,
}

#[derive(Debug)]
pub struct FuzzExecutionConfig<'a, SHM, A, OBS> {
    pub debug_child: bool,
    pub debug_afl: bool,
    pub fuzz_input: FuzzInput<SHM>,
    pub auto_tokens: Option<&'a mut Tokens>,
    pub coverage_map_info: Option<(ShMemId, usize)>,
    pub map_observer: A,
    pub asan_observer_handle: Option<Handle<AsanBacktraceObserver>>,
    pub other_observers: OBS,
}

#[derive(Debug)]
pub struct LspExecutor<S, OBS, SHM> {
    fork_server: NeoForkServer,
    crash_exit_code: Option<i8>,
    timeout: TimeSpec,
    fuzz_input: FuzzInput<SHM>,
    observers: OBS,
    asan_observer_handle: Option<Handle<AsanBacktraceObserver>>,
    _state: PhantomData<S>,
}

impl<S, OBS, A, SHM> LspExecutor<S, (A, OBS), SHM>
where
    SHM: ShMem,
{
    /// Create and initialize a new LSP executor.
    pub fn start<MO>(
        target_info: FuzzTargetInfo,
        mut config: FuzzExecutionConfig<'_, SHM, A, OBS>,
    ) -> Result<Self, libafl::Error>
    where
        S: State + UsesInput<Input = LspInput>,
        MO: MapObserver + Truncate,
        A: Observer<S::Input, S> + AsMut<MO> + AsRef<MO>,
        OBS: ObserversTuple<S::Input, S> + Prepend<A>,
    {
        let args = target_info.args.into_iter().map(|it| it.into()).collect();

        let mut asan_options = vec![
            "detect_odr_violation=0",
            "abort_on_error=1",
            "symbolize=0",
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

        if config.asan_observer_handle.is_some() {
            asan_options.push(const_str::concat!("log_path=", ASAN_LOG_PATH));
        }

        let envs = vec![("ASAN_OPTIONS".into(), asan_options.join(":").into())];

        let opts = NeoForkServerOptions {
            target: target_info.path.as_os_str().to_owned(),
            args,
            envs,
            input_setup: FuzzInputSetup::from(&config.fuzz_input),
            memlimit: 0,
            persistent_fuzzing: target_info.persistent_fuzzing,
            deferred: target_info.defer_fork_server,
            coverage_map_info: config.coverage_map_info,
            afl_debug: config.debug_afl,
            debug_output: config.debug_child,
            kill_signal: target_info.kill_signal,
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
                _ => unreachable!("Garenteed by the match statement above."),
            }
        }

        if matches!(config.fuzz_input, FuzzInput::SharedMemory(_) if !options.shmem_fuzz) {
            Err(libafl::Error::unknown(
                "Target requested sharedmem fuzzing, but you didn't prepare shmem",
            ))?;
        }

        if let (Some(auto_dict), Some(ref auto_dict_content)) =
            (config.auto_tokens, options.autodict)
        {
            auto_dict.parse_autodict(auto_dict_content, auto_dict_content.len());
        }

        let observers = (config.map_observer, config.other_observers);

        Ok(Self {
            fork_server,
            crash_exit_code: target_info.crash_exit_code,
            timeout: target_info.timeout,
            fuzz_input: config.fuzz_input,
            observers,
            asan_observer_handle: config.asan_observer_handle,
            _state: PhantomData,
        })
    }
}

impl<S, OBS, SHM> UsesState for LspExecutor<S, OBS, SHM>
where
    S: State + UsesInput<Input = LspInput>,
{
    type State = S;
}

impl<S, OBS, SHM> HasObservers for LspExecutor<S, OBS, SHM>
where
    S: State + UsesInput<Input = LspInput>,
    OBS: ObserversTuple<S::Input, S>,
{
    type Observers = OBS;

    fn observers(&self) -> RefIndexable<&OBS, OBS> {
        RefIndexable::from(&self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut OBS, OBS> {
        RefIndexable::from(&mut self.observers)
    }
}

impl<EM, Z, S, OBS, SHM> Executor<EM, Z> for LspExecutor<S, OBS, SHM>
where
    S: State + UsesInput<Input = LspInput> + HasExecutions,
    EM: UsesState<State = S>,
    OBS: ObserversTuple<S::Input, S>,
    SHM: ShMem,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut Self::State,
        _mgr: &mut EM,
        input: &Self::Input,
    ) -> Result<ExitKind, libafl::Error> {
        // Setup workspace directory
        let workspace_dir = temp_dir().join(format!("lsp-fuzz-workspace_{}", state.executions()));
        std::fs::create_dir_all(&workspace_dir)?;
        input.setup_source_dir(&workspace_dir)?;

        // Transfer input to the fork server
        let input_bytes = input.request_bytes(&workspace_dir);
        self.fuzz_input.send(&input_bytes)?;

        let (child_pid, status) = self.fork_server.run_child(&self.timeout)?;

        let exit_kind = if let Some(status) = status {
            let exitcode_is_crash = self
                .crash_exit_code
                .filter(|_| libc::WIFEXITED(status))
                .map(|it| libc::WEXITSTATUS(status) as i8 == it)
                .unwrap_or_default();
            if libc::WIFSIGNALED(status) || exitcode_is_crash {
                if let Some(ref handle) = self.asan_observer_handle.as_ref().cloned() {
                    self.update_asan_observer(child_pid, handle)?;
                }
                ExitKind::Crash
            } else {
                ExitKind::Ok
            }
        } else {
            ExitKind::Timeout
        };

        *state.executions_mut() += 1;
        Ok(exit_kind)
    }
}

impl<S, OBS, SHM> LspExecutor<S, OBS, SHM>
where
    S: State + UsesInput<Input = LspInput> + HasExecutions,
    OBS: ObserversTuple<S::Input, S>,
    SHM: ShMem,
{
    fn update_asan_observer(
        &mut self,
        child_pid: Pid,
        handle: &Handle<AsanBacktraceObserver>,
    ) -> Result<(), libafl::Error> {
        let mut observers = self.observers_mut();
        let asan_observer = observers
            .get_mut(handle)
            .afl_context("ASAN handle is attached but ASAN observer not found")?;
        let asan_log_file = format!("{ASAN_LOG_PATH}.{child_pid}");
        if fs::exists(&asan_log_file)? {
            let asan_log = fs::read(&asan_log_file).afl_context("Reading ASAN log file")?;
            let asan_log = String::from_utf8_lossy(&asan_log);
            asan_observer.parse_asan_output(&asan_log);
            fs::remove_file(asan_log_file).afl_context("Fail to cleanup ASAN log file")?;
        }
        Ok(())
    }
}
