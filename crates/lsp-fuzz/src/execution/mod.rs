use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Seek, Write},
    marker::PhantomData,
    mem::{self, transmute},
    os::fd::AsFd,
    path::PathBuf,
};

use fork_server::{FuzzInputSetup, NeoForkServer, NeoForkServerOptions};
use libafl::{
    HasBytesConverter, HasMetadata,
    executors::{Executor, ExitKind, HasObservers},
    inputs::InputToBytes,
    observers::{AsanBacktraceObserver, MapObserver, Observer, ObserversTuple},
    state::HasExecutions,
};
use libafl_bolts::{
    AsSliceMut, HasLen, Named, Truncate,
    fs::InputFile,
    shmem::{ShMem, ShMemId},
    tuples::{MatchName, RefIndexable, type_eq},
};
use nix::{
    sys::{signal::Signal, time::TimeSpec},
    unistd::Pid,
};
use responses::ResponsesObserver;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tracing::info;

use crate::{utf8::UTF8Tokens, utils::AflContext};

pub mod fork_server;
pub mod responses;
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
pub struct FuzzExecutionConfig<'a, SHM, MO, OBS> {
    pub debug_child: bool,
    pub debug_afl: bool,
    pub fuzz_input: FuzzInput<SHM>,
    pub auto_tokens: Option<&'a mut UTF8Tokens>,
    pub coverage_shm_info: (ShMemId, usize),
    pub map_observer: MO,
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
    pub fn start<A>(
        target_info: FuzzTargetInfo,
        mut config: FuzzExecutionConfig<'_, SHM, MO, OBS>,
    ) -> Result<Self, libafl::Error>
    where
        MO: AsRef<A> + AsMut<A>,
        A: Truncate + HasLen + MapObserver,
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
            responses_observer: ResponsesObserver::new(),
            asan_observer: config.asan_observer,
            other_observers: config.other_observers,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Observers<MO, OBS> {
    map_observer: MO,
    asan_observer: Option<AsanBacktraceObserver>,
    responses_observer: ResponsesObserver,
    other_observers: OBS,
}

impl<MO, OBS> MatchName for Observers<MO, OBS>
where
    MO: Named,
    OBS: MatchName,
{
    fn match_name<T>(&self, name: &str) -> Option<&T> {
        if type_eq::<T, MO>() && self.map_observer.name() == name {
            Some(unsafe { transmute::<&MO, &T>(&self.map_observer) })
        } else if let Some(ref asan_observer) = self.asan_observer
            && type_eq::<T, AsanBacktraceObserver>()
            && asan_observer.name() == name
        {
            return Some(unsafe { transmute::<&AsanBacktraceObserver, &T>(asan_observer) });
        } else {
            #[allow(deprecated, reason = "Fallback call")]
            self.other_observers.match_name(name)
        }
    }

    fn match_name_mut<T>(&mut self, name: &str) -> Option<&mut T> {
        if type_eq::<T, MO>() && self.map_observer.name() == name {
            Some(unsafe { transmute::<&mut MO, &mut T>(&mut self.map_observer) })
        } else if let Some(ref mut asan_observer) = self.asan_observer
            && type_eq::<T, AsanBacktraceObserver>()
            && asan_observer.name() == name
        {
            return Some(unsafe { transmute::<&mut AsanBacktraceObserver, &mut T>(asan_observer) });
        } else {
            #[allow(deprecated, reason = "Fallback call")]
            self.other_observers.match_name_mut(name)
        }
    }
}

impl<I, State, MO, OBS> ObserversTuple<I, State> for Observers<MO, OBS>
where
    MO: Observer<I, State>,
    OBS: ObserversTuple<I, State>,
{
    fn pre_exec_all(&mut self, state: &mut State, input: &I) -> Result<(), libafl::Error> {
        self.map_observer.pre_exec(state, input)?;
        if let Some(ref mut asan_observer) = self.asan_observer {
            asan_observer.pre_exec(state, input)?;
        }
        self.other_observers.pre_exec_all(state, input)?;
        Ok(())
    }

    fn post_exec_all(
        &mut self,
        state: &mut State,
        input: &I,
        exit_kind: &ExitKind,
    ) -> Result<(), libafl::Error> {
        self.map_observer.post_exec(state, input, exit_kind)?;
        if let Some(ref mut asan_observer) = self.asan_observer {
            asan_observer.post_exec(state, input, exit_kind)?;
        }
        self.other_observers
            .post_exec_all(state, input, exit_kind)?;
        Ok(())
    }

    fn pre_exec_child_all(&mut self, state: &mut State, input: &I) -> Result<(), libafl::Error> {
        self.map_observer.pre_exec_child(state, input)?;
        if let Some(ref mut asan_observer) = self.asan_observer {
            asan_observer.pre_exec_child(state, input)?;
        }
        self.other_observers.pre_exec_child_all(state, input)?;
        Ok(())
    }

    fn post_exec_child_all(
        &mut self,
        state: &mut State,
        input: &I,
        exit_kind: &ExitKind,
    ) -> Result<(), libafl::Error> {
        self.map_observer.post_exec_child(state, input, exit_kind)?;
        if let Some(ref mut asan_observer) = self.asan_observer {
            asan_observer.post_exec_child(state, input, exit_kind)?;
        }
        self.other_observers
            .post_exec_child_all(state, input, exit_kind)?;
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
    OBS: ObserversTuple<I, State>,
    SHM: ShMem,
    Z: HasBytesConverter,
    Z::Converter: InputToBytes<I>,
{
    fn run_target(
        &mut self,
        fuzzer: &mut Z,
        state: &mut State,
        _mgr: &mut EM,
        input: &I,
    ) -> Result<ExitKind, libafl::Error> {
        // Transfer input to the fork server
        let bytes = fuzzer.converter_mut().to_bytes(input);
        let input_bytes = bytes;
        self.fuzz_input.send(&input_bytes)?;

        self.observers.pre_exec_child_all(state, input)?;

        self.clear_output_capture_file()?;

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
        if exit_kind == ExitKind::Ok {
            self.output_capture_file
                .rewind()
                .afl_context("Rewinding output capture file")?;
            let output_reader = BufReader::new(&mut self.output_capture_file);
            self.observers
                .responses_observer
                .capture_stdout_content(output_reader)
                .afl_context("Capturing target output")?;
            info!(
                "Output captured, {} messages",
                self.observers.responses_observer.captured_messages().len()
            );
        }
        self.observers
            .post_exec_child_all(state, input, &exit_kind)?;
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
