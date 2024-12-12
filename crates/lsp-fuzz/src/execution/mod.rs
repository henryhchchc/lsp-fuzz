use std::{env::temp_dir, fs, marker::PhantomData, mem, path::Path};

use fork_server::{FuzzInputSetup, NeoForkServer};
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
use tracing::warn;

use crate::{
    lsp_input::LspInput,
    utils::{OptionExt, ResultExt},
};

mod fork_server;

const ASAN_LOG_PATH: &str = "/tmp/asan";

#[derive(Debug)]
pub enum FuzzInput<SHM> {
    Stdin(InputFile),
    File(InputFile),
    SharedMemory(SHM),
}

impl<SHM: ShMem> FuzzInput<SHM> {
    const SHM_FUZZ_HEADER_SIZE: usize = mem::size_of::<u32>();

    pub fn feed(&mut self, input_bytes: &[u8]) -> Result<(), libafl::Error> {
        match self {
            FuzzInput::Stdin(file) | FuzzInput::File(file) => {
                file.write_buf(input_bytes)?;
            }
            FuzzInput::SharedMemory(shmem) => {
                std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
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
                std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
            }
        }
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
pub struct LspExecutor<S, OT, SHM> {
    fork_server: NeoForkServer,
    crash_exit_code: Option<i8>,
    timeout: TimeSpec,
    fuzz_input: FuzzInput<SHM>,
    observers: OT,
    asan_observer_handle: Option<Handle<AsanBacktraceObserver>>,
    _state: PhantomData<S>,
}

impl<S, OT, A, SHM> LspExecutor<S, (A, OT), SHM>
where
    SHM: ShMem,
{
    #[allow(clippy::too_many_arguments, reason = "To be refactored later")]
    pub fn new<MO>(
        fuzz_target: &Path,
        target_args: Vec<String>,
        crash_exit_code: Option<i8>,
        timeout: TimeSpec,
        debug_child: bool,
        debug_afl: bool,
        kill_signal: Signal,
        fuzz_input: FuzzInput<SHM>,
        is_persistent: bool,
        is_deferred_fork_server: bool,
        auto_tokens: Option<&mut Tokens>,
        coverage_map_info: Option<(ShMemId, usize)>,
        mut map_observer: A,
        asan_observer_handle: Option<Handle<AsanBacktraceObserver>>,
        other_observers: OT,
    ) -> Result<Self, libafl::Error>
    where
        S: State + UsesInput<Input = LspInput>,
        MO: MapObserver + Truncate,
        A: Observer<S::Input, S> + AsMut<MO> + AsRef<MO>,
        OT: ObserversTuple<S::Input, S> + Prepend<A>,
    {
        let args = target_args.into_iter().map(|it| it.into()).collect();

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

        if asan_observer_handle.is_some() {
            asan_options.push(const_str::concat!("log_path=", ASAN_LOG_PATH));
        }

        let envs = vec![("ASAN_OPTIONS".into(), asan_options.join(":").into())];

        let mut fork_server = fork_server::NeoForkServer::new(
            fuzz_target.as_os_str().to_owned(),
            args,
            envs,
            FuzzInputSetup::from(&fuzz_input),
            0,
            is_persistent,
            is_deferred_fork_server,
            coverage_map_info,
            debug_afl,
            debug_child,
            kill_signal,
        )?;

        fork_server::initialize(
            &mut fork_server,
            &mut map_observer,
            &fuzz_input,
            auto_tokens,
        )?;

        let observers = (map_observer, other_observers);

        Ok(Self {
            fork_server,
            crash_exit_code,
            timeout,
            fuzz_input,
            observers,
            asan_observer_handle,
            _state: PhantomData,
        })
    }
}

impl<S, OT, SHM> UsesState for LspExecutor<S, OT, SHM>
where
    S: State + UsesInput<Input = LspInput>,
{
    type State = S;
}

impl<S, OT, SHM> HasObservers for LspExecutor<S, OT, SHM>
where
    S: State + UsesInput<Input = LspInput>,
    OT: ObserversTuple<S::Input, S>,
{
    type Observers = OT;

    fn observers(&self) -> RefIndexable<&OT, OT> {
        RefIndexable::from(&self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut OT, OT> {
        RefIndexable::from(&mut self.observers)
    }
}

impl<EM, Z, S, OT, SHM> Executor<EM, Z> for LspExecutor<S, OT, SHM>
where
    S: State + UsesInput<Input = LspInput> + HasExecutions,
    EM: UsesState<State = S>,
    Z: UsesState<State = S>,
    OT: ObserversTuple<S::Input, S>,
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
        self.fuzz_input.feed(&input_bytes)?;

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
        // std::fs::remove_dir_all(&workspace_dir)?;
        *state.executions_mut() += 1;
        Ok(exit_kind)
    }
}

impl<S, OT, SHM> LspExecutor<S, OT, SHM>
where
    S: State + UsesInput<Input = LspInput> + HasExecutions,
    OT: ObserversTuple<S::Input, S>,
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
