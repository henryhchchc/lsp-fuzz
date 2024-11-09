use std::{env::temp_dir, marker::PhantomData, path::Path};

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
    tuples::{Prepend, RefIndexable},
    Truncate,
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
pub struct LspExecutor<S, OT> {
    fork_server: ForkServer,
    crash_exit_code: Option<i8>,
    kill_signal: Signal,
    timeout: TimeSpec,
    input_file: InputFile,
    observers: OT,
    _state: PhantomData<S>,
}

const FS_NEW_OPT_MAPSIZE: i32 = 1 << 0;
const FS_NEW_OPT_SHDMEM_FUZZ: i32 = 1 << 1;
const FS_NEW_OPT_AUTODICT: i32 = 1 << 11;

impl<S, OT, A> LspExecutor<S, (A, OT)> {
    #[allow(clippy::too_many_arguments, reason = "To be refactored later")]
    pub fn new<MO>(
        fuzz_target: &Path,
        mut target_args: Vec<String>,
        crash_exit_code: Option<i8>,
        timeout: TimeSpec,
        debug_child: bool,
        kill_signal: Signal,
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

        let mut fork_server = ForkServer::with_kill_signal(
            fuzz_target.as_os_str().to_owned(),
            args,
            envs,
            input_file.as_raw_fd(),
            true,
            0,
            false,
            false,
            false,
            Some(map_observer.as_ref().len()),
            debug_child,
            kill_signal,
        )?;

        // Initial handshake, read 4-bytes hello message from the forkserver.
        let handshake_msg = fork_server
            .read_st()
            .afl_context("Oops the fork server fucked up.")?;

        fork_server::check_handshake_error_bits(handshake_msg)?;
        fork_server::check_version(handshake_msg)?;

        // Send back handshake response to the forkserver.
        let handshake_response = (handshake_msg as u32 ^ 0xffffffff) as i32;
        fork_server
            .write_ctl(handshake_response)
            .afl_context("Fail to write handshake response to forkserver")?;

        let fsrv_options = fork_server
            .read_st()
            .afl_context("Fail to read options from forkserver")?;

        if fsrv_options & FS_NEW_OPT_MAPSIZE == FS_NEW_OPT_MAPSIZE {
            let fsrv_map_size = fork_server
                .read_st()
                .afl_context("Failed to read map size from forkserver")?;

            map_observer.as_mut().truncate(fsrv_map_size as usize);
            if map_observer.as_ref().len() < fsrv_map_size as usize {
                Err(libafl::Error::illegal_argument(format!(
                    "The map size is too small. {fsrv_map_size} is required for the target."
                )))?;
            }
            info!(new_size = fsrv_map_size, "Coverage map truncated");
        };

        if fsrv_options & FS_NEW_OPT_SHDMEM_FUZZ != 0 {
            Err(libafl::Error::unknown(
                "Target requested sharedmem fuzzing, but you didn't prepare shmem",
            ))?;
        }

        if fsrv_options & FS_NEW_OPT_AUTODICT != 0 {
            // Here unlike shmem input fuzzing, we are forced to read things
            // hence no self.autotokens.is_some() to check if we proceed
            let autotokens_size = fork_server
                .read_st()
                .afl_context("Failed to read autotokens size from forkserver")?;

            let tokens_size_max = 0xffffff;

            if !(2..=tokens_size_max).contains(&autotokens_size) {
                let message = format!("Autotokens size is incorrect, expected 2 to {tokens_size_max} (inclusive), but got {autotokens_size}. Make sure your afl-cc verison is up to date.");
                Err(libafl::Error::illegal_state(message))?;
            }
            info!(size = autotokens_size, "AUTODICT detected.");
            let auto_tokens_buf = fork_server
                .read_st_of_len(autotokens_size as usize)
                .afl_context("Failed to load autotokens")?;
            if let Some(t) = auto_tokens {
                info!("Updating autotokens.");
                t.parse_autodict(&auto_tokens_buf, autotokens_size as usize);
            }
        }

        let aflx = fork_server
            .read_st()
            .afl_context("Reading from forkserver failed")?;

        if aflx != handshake_msg {
            let message =
                format!("Error in forkserver communication ({aflx:?}=>{handshake_msg:?})");
            Err(libafl::Error::unknown(message))?;
        }

        let observers = (map_observer, other_observers);

        Ok(Self {
            fork_server,
            crash_exit_code,
            kill_signal,
            timeout,
            observers,
            input_file,
            _state: PhantomData,
        })
    }
}

impl<S, OT> UsesState for LspExecutor<S, OT>
where
    S: State + UsesInput<Input = LspInput>,
{
    type State = S;
}

impl<S, OT> HasObservers for LspExecutor<S, OT>
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

impl<EM, Z, S, OT> Executor<EM, Z> for LspExecutor<S, OT>
where
    S: State + UsesInput<Input = LspInput> + HasExecutions,
    EM: UsesState<State = S>,
    Z: UsesState<State = S>,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut Self::State,
        _mgr: &mut EM,
        input: &Self::Input,
    ) -> Result<ExitKind, libafl::Error> {
        let workspace_dir = temp_dir().join(format!("lsp-fuzz-workspace_{}", state.executions()));
        std::fs::create_dir_all(&workspace_dir)?;
        input.setup_source_dir(&workspace_dir)?;
        let input_bytes = input.request_bytes(&workspace_dir);
        let input_size = input_bytes.as_slice().len();
        self.input_file
            .write_buf(&input_bytes.as_slice()[..input_size])?;

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
        // std::fs::remove_dir_all(&workspace_dir)?;
        *state.executions_mut() += 1;
        Ok(exit_kind)
    }
}
