use std::{env::temp_dir, marker::PhantomData, path::Path};

use libafl::{
    inputs::{HasMutatorBytes, UsesInput},
    prelude::{
        Executor, ExitKind, Forkserver, HasObservers, MapObserver, Observer, ObserversTuple,
        Tokens, UsesObservers,
    },
    state::{HasExecutions, State, UsesState},
};
use libafl_bolts::{
    current_nanos, fs::InputFile, prelude::RefIndexable, tuples::Prepend, Truncate,
};
use nix::{
    sys::{
        signal::{kill, Signal},
        time::TimeSpec,
    },
    unistd::Pid,
};
use tracing::info;

use crate::inputs::LspInput;

#[derive(Debug)]
pub struct LspExecutor<S, OT> {
    fork_server: Forkserver,
    crash_exit_code: Option<i8>,
    kill_signal: Signal,
    timeout: TimeSpec,
    input_file: InputFile,
    observers: OT,
    _state: PhantomData<S>,
}

#[allow(clippy::cast_possible_wrap)]
const FS_NEW_ERROR: i32 = 0xeffe0000_u32 as i32;

const FS_NEW_VERSION_MIN: u32 = 1;
const FS_NEW_VERSION_MAX: u32 = 1;

#[allow(clippy::cast_possible_wrap)]
const FS_NEW_OPT_MAPSIZE: i32 = 1_u32 as i32;

#[allow(clippy::cast_possible_wrap)]
const FS_NEW_OPT_SHDMEM_FUZZ: i32 = 2_u32 as i32;

#[allow(clippy::cast_possible_wrap)]
const FS_NEW_OPT_AUTODICT: i32 = 0x00000800_u32 as i32;

#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_MAP_SIZE: i32 = 1_u32 as i32;
#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_MAP_ADDR: i32 = 2_u32 as i32;
#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_SHM_OPEN: i32 = 4_u32 as i32;
#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_SHMAT: i32 = 8_u32 as i32;
#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_MMAP: i32 = 16_u32 as i32;
#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_OLD_CMPLOG: i32 = 32_u32 as i32;
#[allow(clippy::cast_possible_wrap)]
const FS_ERROR_OLD_CMPLOG_QEMU: i32 = 64_u32 as i32;

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
        A: Observer<S> + AsMut<MO>,
        OT: ObserversTuple<S> + Prepend<A, PreprendResult = OT>,
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

        let mut fork_server = Forkserver::with_kill_signal(
            fuzz_target.as_os_str().to_owned(),
            args,
            Vec::default(),
            input_file.as_raw_fd(),
            true,
            0,
            false,
            false,
            debug_child,
            kill_signal,
        )?;

        let (rlen, version_status) = fork_server.read_st()?; // Initial handshake, read 4-bytes hello message from the forkserver.

        if rlen != 4 {
            return Err(libafl::Error::unknown(
                "Failed to start a forkserver".to_string(),
            ));
        }

        if (version_status & FS_NEW_ERROR) == FS_NEW_ERROR {
            report_error_and_exit(version_status & 0x0000ffff)?;
        }

        if is_old_forkserver(version_status) {
            return Err(libafl::Error::unknown(
                "Old fork server model is used by the target, it is nolonger supportted".to_owned(),
            ));
        } else {
            let keep = version_status;
            let version: u32 = version_status as u32 - 0x41464c00_u32;
            match version {
                0 => {
                    return Err(libafl::Error::unknown(
                            "Fork server version is not assigned, this should not happen. Recompile target.",
                        ));
                }
                FS_NEW_VERSION_MIN..=FS_NEW_VERSION_MAX => {
                    // good, do nothing
                }
                _ => {
                    return Err(libafl::Error::unknown(
                        "Fork server version is not supported. Recompile the target.",
                    ));
                }
            }

            let xored_status = (version_status as u32 ^ 0xffffffff) as i32;

            let send_len = fork_server.write_ctl(xored_status)?;
            if send_len != 4 {
                return Err(libafl::Error::unknown(
                    "Writing to forkserver failed.".to_string(),
                ));
            }

            info!(
                "All right - new fork server model version {} is up",
                version
            );

            let (read_len, status) = fork_server.read_st()?;
            if read_len != 4 {
                return Err(libafl::Error::unknown(
                    "Reading from forkserver failed.".to_string(),
                ));
            }

            if status & FS_NEW_OPT_MAPSIZE == FS_NEW_OPT_MAPSIZE {
                let (read_len, fsrv_map_size) = fork_server.read_st()?;
                if read_len != 4 {
                    return Err(libafl::Error::unknown(
                        "Failed to read map size from forkserver".to_string(),
                    ));
                }
                map_observer.as_mut().truncate(fsrv_map_size as usize);
                info!(new_size = fsrv_map_size, "Coverage map truncated");
            }

            if status & FS_NEW_OPT_SHDMEM_FUZZ != 0 {
                return Err(libafl::Error::unknown(
                    "Target requested sharedmem fuzzing, but you didn't prepare shmem",
                ));
            }

            if status & FS_NEW_OPT_AUTODICT != 0 {
                // Here unlike shmem input fuzzing, we are forced to read things
                // hence no self.autotokens.is_some() to check if we proceed
                let (read_len, autotokens_size) = fork_server.read_st()?;
                if read_len != 4 {
                    return Err(libafl::Error::unknown(
                        "Failed to read autotokens size from forkserver".to_string(),
                    ));
                }

                let tokens_size_max = 0xffffff;

                if !(2..=tokens_size_max).contains(&autotokens_size) {
                    return Err(libafl::Error::illegal_state(
                                format!("Autotokens size is incorrect, expected 2 to {tokens_size_max} (inclusive), but got {autotokens_size}. Make sure your afl-cc verison is up to date."),
                            ));
                }
                info!(size = autotokens_size, "AUTODICT detected.");
                let (rlen, buf) = fork_server.read_st_size(autotokens_size as usize)?;

                if rlen != autotokens_size as usize {
                    return Err(libafl::Error::unknown(
                        "Failed to load autotokens".to_string(),
                    ));
                }
                if let Some(t) = auto_tokens {
                    info!("Updating autotokens.");
                    t.parse_autodict(&buf, autotokens_size as usize);
                }
            }

            let (read_len, aflx) = fork_server.read_st()?;
            if read_len != 4 {
                return Err(libafl::Error::unknown(
                    "Reading from forkserver failed".to_string(),
                ));
            }

            if aflx != keep {
                return Err(libafl::Error::unknown(format!(
                    "Error in forkserver communication ({aflx:?}=>{keep:?})",
                )));
            }
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

impl<S, OT> UsesObservers for LspExecutor<S, OT>
where
    S: State + UsesInput<Input = LspInput>,
    OT: ObserversTuple<S>,
{
    type Observers = OT;
}

impl<S, OT> HasObservers for LspExecutor<S, OT>
where
    S: State + UsesInput<Input = LspInput>,
    OT: ObserversTuple<S>,
{
    fn observers(&self) -> RefIndexable<&Self::Observers, Self::Observers> {
        RefIndexable::from(&self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut Self::Observers, Self::Observers> {
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
        *state.executions_mut() += 1;
        let mut exit_kind = ExitKind::Ok;
        let last_run_timed_out = self.fork_server.last_run_timed_out_raw();
        let input_bytes = input.bytes().to_vec();
        let input_size = input_bytes.as_slice().len();
        self.input_file
            .write_buf(&input_bytes.as_slice()[..input_size])?;

        let send_len = self.fork_server.write_ctl(last_run_timed_out)?;
        self.fork_server.set_last_run_timed_out(false);
        if send_len != 4 {
            return Err(libafl::Error::unknown(
                "Unable to request new process from fork server (OOM?)".to_string(),
            ));
        }
        let (recv_pid_len, pid) = self.fork_server.read_st()?;
        if recv_pid_len != 4 {
            return Err(libafl::Error::unknown(
                "Unable to request new process from fork server (OOM?)".to_string(),
            ));
        }
        if pid <= 0 {
            return Err(libafl::Error::unknown(
                "Fork server is misbehaving (OOM?)".to_string(),
            ));
        }
        self.fork_server.set_child_pid(Pid::from_raw(pid));
        if let Some(status) = self.fork_server.read_st_timed(&self.timeout)? {
            self.fork_server.set_status(status);
            let exitcode_is_crash = if let Some(crash_exitcode) = self.crash_exit_code {
                (libc::WEXITSTATUS(self.fork_server.status()) as i8) == crash_exitcode
            } else {
                false
            };
            if libc::WIFSIGNALED(self.fork_server.status()) || exitcode_is_crash {
                exit_kind = ExitKind::Crash;
            }
        } else {
            self.fork_server.set_last_run_timed_out(true);
            // We need to kill the child in case he has timed out, or we can't get the correct
            // pid in the next call to self.executor.forkserver_mut().read_st()?
            let _ = kill(self.fork_server.child_pid(), self.kill_signal);
            let (recv_status_len, _) = self.fork_server.read_st()?;
            if recv_status_len != 4 {
                return Err(libafl::Error::unknown(
                    "Could not kill timed-out child".to_string(),
                ));
            }
            exit_kind = ExitKind::Timeout;
        }
        if !libc::WIFSTOPPED(self.fork_server.status()) {
            self.fork_server.reset_child_pid();
        }
        Ok(exit_kind)
    }
}

// Stolen from libafl
fn report_error_and_exit(status: i32) -> Result<(), libafl::Error> {
    /* Report on the error received via the forkserver controller and exit */
    match status {
    FS_ERROR_MAP_SIZE =>
        Err(libafl::Error::unknown(
            "AFL_MAP_SIZE is not set and fuzzing target reports that the required size is very large. Solution: Run the fuzzing target stand-alone with the environment variable AFL_DEBUG=1 set and set the value for __afl_final_loc in the AFL_MAP_SIZE environment variable for afl-fuzz.".to_string())),
    FS_ERROR_MAP_ADDR =>
        Err(libafl::Error::unknown(
            "the fuzzing target reports that hardcoded map address might be the reason the mmap of the shared memory failed. Solution: recompile the target with either afl-clang-lto and do not set AFL_LLVM_MAP_ADDR or recompile with afl-clang-fast.".to_string())),
    FS_ERROR_SHM_OPEN =>
        Err(libafl::Error::unknown("the fuzzing target reports that the shm_open() call failed.".to_string())),
    FS_ERROR_SHMAT =>
        Err(libafl::Error::unknown("the fuzzing target reports that the shmat() call failed.".to_string())),
    FS_ERROR_MMAP =>
        Err(libafl::Error::unknown("the fuzzing target reports that the mmap() call to the shared memory failed.".to_string())),
    FS_ERROR_OLD_CMPLOG =>
        Err(libafl::Error::unknown(
            "the -c cmplog target was instrumented with an too old AFL++ version, you need to recompile it.".to_string())),
    FS_ERROR_OLD_CMPLOG_QEMU =>
        Err(libafl::Error::unknown("The AFL++ QEMU/FRIDA loaders are from an older version, for -c you need to recompile it.".to_string())),
    _ =>
        Err(libafl::Error::unknown(format!("unknown error code {status} from fuzzing target!"))),
    }
}

fn is_old_forkserver(version_status: i32) -> bool {
    !(0x41464c00..0x41464cff).contains(&version_status)
}
