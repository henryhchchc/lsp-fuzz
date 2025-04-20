use std::{collections::HashMap, fs, marker::PhantomData, mem, path::PathBuf};

use fork_server::{FuzzInputSetup, NeoForkServer, NeoForkServerOptions};
use libafl::{
    HasMetadata,
    executors::{Executor, ExitKind, HasObservers},
    observers::{AsanBacktraceObserver, MapObserver, Observer, ObserversTuple},
    state::HasExecutions,
};
use libafl_bolts::{
    AsSliceMut, Truncate,
    fs::InputFile,
    shmem::{ShMem, ShMemId},
    tuples::{Prepend, RefIndexable},
};
use nix::{
    sys::{signal::Signal, time::TimeSpec},
    unistd::Pid,
};
use tracing::info;
use workspace_observer::CurrentWorkspaceMetadata;

use crate::{lsp_input::LspInput, utf8::UTF8Tokens, utils::AflContext};

pub mod fork_server;
pub mod sanitizers;
mod test;
pub mod workspace_observer;

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
        use core::sync::atomic::{Ordering, compiler_fence};

        if shmem.len() < input_bytes.len() + Self::SHM_FUZZ_HEADER_SIZE {
            Err(libafl::Error::unknown(
                "The shared memory is too small for the input.",
            ))?;
        }
        let input_size = u32::try_from(input_bytes.len())
            .afl_context("The length of input bytes cannot fit into u32")?;
        let input_size_encoded = input_size.to_ne_bytes();

        compiler_fence(Ordering::Acquire);
        let shmem_slice = shmem.as_slice_mut();
        shmem_slice[..Self::SHM_FUZZ_HEADER_SIZE].copy_from_slice(&input_size_encoded);
        let input_body_range =
            Self::SHM_FUZZ_HEADER_SIZE..(Self::SHM_FUZZ_HEADER_SIZE + input_bytes.len());
        shmem_slice[input_body_range].copy_from_slice(input_bytes);
        compiler_fence(Ordering::Release);

        Ok(())
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
    pub env: HashMap<String, String>,
}

#[derive(Debug)]
pub struct FuzzExecutionConfig<'a, SHM, A, OBS> {
    pub debug_child: bool,
    pub debug_afl: bool,
    pub fuzz_input: FuzzInput<SHM>,
    pub auto_tokens: Option<&'a mut UTF8Tokens>,
    pub coverage_shm_info: Option<(ShMemId, usize)>,
    pub map_observer: A,
    pub asan_observer: Option<AsanBacktraceObserver>,
    pub other_observers: OBS,
}

#[derive(Debug)]
pub struct LspExecutor<State, OBS, SHM> {
    fork_server: NeoForkServer,
    crash_exit_code: Option<i8>,
    timeout: TimeSpec,
    fuzz_input: FuzzInput<SHM>,
    asan_observer: Option<AsanBacktraceObserver>,
    observers: OBS,
    _state: PhantomData<State>,
}

impl<State, OBS, A, SHM> LspExecutor<State, (A, OBS), SHM>
where
    SHM: ShMem,
{
    /// Create and initialize a new LSP executor.
    pub fn start<MO>(
        target_info: FuzzTargetInfo,
        mut config: FuzzExecutionConfig<'_, SHM, A, OBS>,
    ) -> Result<Self, libafl::Error>
    where
        MO: MapObserver + Truncate,
        A: Observer<LspInput, State> + AsMut<MO> + AsRef<MO>,
        OBS: ObserversTuple<LspInput, State> + Prepend<A>,
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
            "detect_stack_use_after_return=1",
            "check_initialization_order=0",
            "detect_leaks=1",
            "malloc_context_size=0",
        ];

        if config.asan_observer.is_some() {
            asan_options.push(const_str::concat!("log_path=", ASAN_LOG_PATH));
        }

        let mut envs = vec![("ASAN_OPTIONS".into(), asan_options.join(":").into())];

        envs.extend(
            target_info
                .env
                .into_iter()
                .map(|(k, v)| (k.into(), v.into())),
        );

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

        let observers = (config.map_observer, config.other_observers);

        Ok(Self {
            fork_server,
            crash_exit_code: target_info.crash_exit_code,
            timeout: target_info.timeout,
            fuzz_input: config.fuzz_input,
            observers,
            asan_observer: config.asan_observer,
            _state: PhantomData,
        })
    }
}

impl<State, OBS, SHM> HasObservers for LspExecutor<State, OBS, SHM>
where
    OBS: ObserversTuple<LspInput, State>,
{
    type Observers = OBS;

    fn observers(&self) -> RefIndexable<&OBS, OBS> {
        RefIndexable::from(&self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut OBS, OBS> {
        RefIndexable::from(&mut self.observers)
    }
}

impl<EM, Z, State, OBS, SHM> Executor<EM, LspInput, State, Z> for LspExecutor<State, OBS, SHM>
where
    State: HasExecutions + HasMetadata,
    OBS: ObserversTuple<LspInput, State>,
    SHM: ShMem,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut State,
        _mgr: &mut EM,
        input: &LspInput,
    ) -> Result<ExitKind, libafl::Error> {
        let workspace_dir = state
            .metadata::<CurrentWorkspaceMetadata>()
            .afl_context("No current working dir available")?
            .path();
        // Transfer input to the fork server
        let input_bytes = input.request_bytes(workspace_dir);
        self.fuzz_input.send(&input_bytes)?;

        self.observers.pre_exec_child_all(state, input)?;
        let (child_pid, status) = self.fork_server.run_child(&self.timeout)?;

        let exit_kind = if let Some(status) = status {
            let exitcode_is_crash = self
                .crash_exit_code
                .filter(|_| libc::WIFEXITED(status))
                .map(|it| libc::WEXITSTATUS(status) as i8 == it)
                .unwrap_or_default();
            if libc::WIFSIGNALED(status) || exitcode_is_crash {
                ExitKind::Crash
            } else {
                ExitKind::Ok
            }
        } else {
            ExitKind::Timeout
        };
        self.observers
            .post_exec_child_all(state, input, &exit_kind)?;
        if exit_kind == ExitKind::Crash {
            if let Some(ref mut asan_observer) = self.asan_observer {
                if let Some(ref asan_log_content) = read_asan_log(child_pid)? {
                    let log_content = String::from_utf8_lossy(asan_log_content);
                    asan_observer.parse_asan_output(log_content.as_ref());
                }
            }
        }

        *state.executions_mut() += 1;
        Ok(exit_kind)
    }
}

fn read_asan_log(child_pid: Pid) -> Result<Option<Vec<u8>>, libafl::Error> {
    let asan_log_file = format!("{ASAN_LOG_PATH}.{child_pid}");
    let log = if fs::exists(&asan_log_file)? {
        let asan_log = fs::read(&asan_log_file).afl_context("Reading ASAN log file")?;
        fs::remove_file(asan_log_file).afl_context("Fail to cleanup ASAN log file")?;
        Some(asan_log)
    } else {
        None
    };
    Ok(log)
}
