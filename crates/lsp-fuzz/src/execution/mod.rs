use std::{env::temp_dir, marker::PhantomData, mem, path::Path};

use libafl::{
    executors::{Executor, ExitKind, Forkserver as ForkServer, HasObservers},
    inputs::UsesInput,
    mutators::Tokens,
    observers::{MapObserver, Observer, ObserversTuple},
    state::{HasExecutions, State, UsesState},
};
use libafl_bolts::{
    current_nanos,
    fs::InputFile,
    shmem::ShMem,
    tuples::{Prepend, RefIndexable},
    AsSliceMut, Truncate,
};
use nix::{
    errno::Errno,
    sys::{
        signal::{kill, Signal},
        time::TimeSpec,
    },
    unistd::Pid,
};
use tracing::info;

use crate::{lsp_input::LspInput, utils::ResultExt};

mod fork_server;

#[derive(Debug)]
pub struct LspExecutor<S, OT, SHM> {
    fork_server: ForkServer,
    crash_exit_code: Option<i8>,
    kill_signal: Signal,
    timeout: TimeSpec,
    input_file: InputFile,
    test_case_shmem: Option<SHM>,
    observers: OT,
    _state: PhantomData<S>,
}

impl<S, OT, A, SHM> LspExecutor<S, (A, OT), SHM>
where
    SHM: ShMem,
{
    #[allow(clippy::too_many_arguments, reason = "To be refactored later")]
    pub fn new<MO>(
        fuzz_target: &Path,
        mut target_args: Vec<String>,
        crash_exit_code: Option<i8>,
        timeout: TimeSpec,
        debug_child: bool,
        kill_signal: Signal,
        test_case_shmem: Option<SHM>,
        is_persistent: bool,
        is_deferred_fork_server: bool,
        auto_tokens: Option<&mut Tokens>,
        mut map_observer: A,
        other_observers: OT,
    ) -> Result<Self, libafl::Error>
    where
        S: State + UsesInput<Input = LspInput>,
        MO: MapObserver + Truncate,
        A: Observer<S::Input, S> + AsMut<MO> + AsRef<MO>,
        OT: ObserversTuple<S::Input, S> + Prepend<A>,
    {
        let filename = format!("lsp-fuzz-input_{}", current_nanos());
        let input_file_path = temp_dir().join(filename);
        let input_file = InputFile::create(input_file_path)?;
        info!(path = %input_file.path.display(), "Created input file");

        target_args.iter_mut().for_each(|arg| {
            if arg == "@@" {
                *arg = input_file.path.to_string_lossy().to_string();
            }
        });
        let args = target_args.into_iter().map(|it| it.into()).collect();

        let asan_options = [
            "detect_odr_violation=0",
            "abort_on_error=1",
            "symbolize=0",
            "allocator_may_return_null=1",
            "handle_segv=0",
            "handle_sigbus=0",
            "handle_abort=0",
            "handle_sigfpe=0",
            "handle_sigill=0",
            "detect_stack_use_after_return=0",
            "check_initialization_order=0",
            "detect_leaks=0",
            "malloc_context_size=0",
        ]
        .join(":")
        .into();

        let envs = vec![("ASAN_OPTIONS".into(), asan_options)];

        // This must be done before the fork server is created
        if let Some(ref shmem) = test_case_shmem {
            const AFL_TEST_CASE_SHM_ID_ENV: &str = "__AFL_SHM_FUZZ_ID";
            shmem.write_to_env(AFL_TEST_CASE_SHM_ID_ENV)?;
        }

        let mut fork_server = ForkServer::with_kill_signal(
            fuzz_target.as_os_str().to_owned(),
            args,
            envs,
            input_file.as_raw_fd(),
            test_case_shmem.is_none(),
            0,
            is_persistent,
            is_deferred_fork_server,
            false,
            Some(map_observer.as_ref().len()),
            debug_child,
            kill_signal,
        )?;

        fork_server::initialize(
            &mut fork_server,
            &mut map_observer,
            &test_case_shmem,
            auto_tokens,
        )?;

        let observers = (map_observer, other_observers);

        Ok(Self {
            fork_server,
            crash_exit_code,
            kill_signal,
            timeout,
            test_case_shmem,
            observers,
            input_file,
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
        if let Some(shmem) = self.test_case_shmem.as_mut() {
            write_shm_input(shmem, &input_bytes)?;
        } else {
            self.input_file.write_buf(&input_bytes)?;
        }

        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);

        // Run the fuzzing target
        let last_run_timed_out = self.fork_server.last_run_timed_out_raw();
        self.fork_server
            .write_ctl(last_run_timed_out)
            .afl_context("Oops the fork server is dead.")?;

        self.fork_server.set_last_run_timed_out(false);
        let child_pid = self
            .fork_server
            .read_st()
            .afl_context("Fail to get child PID from fork server")?;

        if child_pid <= 0 {
            Err(libafl::Error::unknown(
                "Get an invalid PID from fork server.",
            ))?;
        }
        self.fork_server.set_child_pid(Pid::from_raw(child_pid));
        let exit_kind = if let Some(status) = self.fork_server.read_st_timed(&self.timeout)? {
            self.fork_server.set_status(status);
            let exitcode_is_crash = self
                .crash_exit_code
                .map(|it| (libc::WEXITSTATUS(self.fork_server.status()) as i8) == it)
                .unwrap_or_default();
            if libc::WIFSIGNALED(self.fork_server.status()) || exitcode_is_crash {
                ExitKind::Crash
            } else {
                ExitKind::Ok
            }
        } else {
            self.fork_server.set_last_run_timed_out(true);
            // We need to kill the child in case he has timed out, or we can't get the correct
            // pid in the next call to self.executor.forkserver_mut().read_st()?
            match kill(self.fork_server.child_pid(), self.kill_signal) {
                Ok(_) | Err(Errno::ESRCH) => {
                    // It is OK if the child terminated before we could kill it
                }
                Err(errno) => {
                    let message =
                        format!("Oops we could not kill timed-out child: {}", errno.desc());
                    Err(libafl::Error::unknown(message))?;
                }
            }
            if self.fork_server.read_st().is_err() {
                return Err(libafl::Error::unknown(
                    "Could not kill timed-out child".to_string(),
                ));
            }
            ExitKind::Timeout
        };
        if !libc::WIFSTOPPED(self.fork_server.status()) {
            self.fork_server.reset_child_pid();
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
        // std::fs::remove_dir_all(&workspace_dir)?;
        *state.executions_mut() += 1;
        Ok(exit_kind)
    }
}

fn write_shm_input<SHM>(shmem: &mut SHM, input_bytes: &[u8]) -> Result<(), libafl::Error>
where
    SHM: ShMem,
{
    const SHM_FUZZ_HEADER_SIZE: usize = mem::size_of::<u32>();
    if shmem.len() < input_bytes.len() + SHM_FUZZ_HEADER_SIZE {
        Err(libafl::Error::unknown(
            "The shared memory is too small for the input.",
        ))?;
    }
    let input_size = u32::try_from(input_bytes.len())
        .afl_context("The length of input bytes cannot fit into u32")?;
    let input_size_encoded = input_size.to_ne_bytes();
    let shmem_slice = shmem.as_slice_mut();
    shmem_slice[..SHM_FUZZ_HEADER_SIZE].copy_from_slice(&input_size_encoded);
    let input_body_range = SHM_FUZZ_HEADER_SIZE..(SHM_FUZZ_HEADER_SIZE + input_bytes.len());
    shmem_slice[input_body_range].copy_from_slice(input_bytes);
    Ok(())
}

impl<S, OT, SHM> Drop for LspExecutor<S, OT, SHM> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.input_file.path);
    }
}
