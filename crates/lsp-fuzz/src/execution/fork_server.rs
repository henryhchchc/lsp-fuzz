use core::slice;
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, BorrowedFd, RawFd},
        unix::process::CommandExt,
    },
    process::{self, Child, Command, Stdio},
};

use bitflags::bitflags;
use libafl::executors::forkserver::ConfigTarget;
use libafl_bolts::shmem::{ShMem, ShMemId};
use nix::{
    errno::Errno,
    sys::{
        select::FdSet,
        signal::{SigSet, Signal},
        time::TimeSpec,
    },
    unistd::Pid,
};
use os_pipe::{PipeReader, PipeWriter};
use tracing::{debug, info, warn};

use crate::utils::{OptionExt, ResultExt};

use super::FuzzInput;

#[derive(Debug)]
pub enum FuzzInputSetup {
    Stdin(RawFd),
    FileArg,
    SharedMemory(ShMemId, usize),
}

const FS_NEW_OPT_MAPSIZE: i32 = 1 << 0;
const FS_NEW_OPT_SHDMEM_FUZZ: i32 = 1 << 1;
const FS_NEW_OPT_AUTODICT: i32 = 1 << 11;

bitflags! {
    struct ForkServerFlags: i32 {
        const MAP_SIZE = FS_NEW_OPT_MAPSIZE;
        const SHMEM_FUZZ = FS_NEW_OPT_SHDMEM_FUZZ;
        const AUTODICT = FS_NEW_OPT_AUTODICT;
    }
}

#[derive(Debug)]
pub struct ForkServerOptions {
    pub map_size: Option<usize>,
    pub shmem_fuzz: bool,
    pub autodict: Option<Vec<u8>>,
}

impl FuzzInputSetup {
    pub fn setup_child_cmd(&self, child_cmd: &mut Command) {
        match self {
            &FuzzInputSetup::Stdin(input_filefd) => {
                let bind_stdin = move || {
                    nix::unistd::dup2(input_filefd, libc::STDIN_FILENO)
                        .map_err(|_| io::Error::last_os_error())?;
                    Ok(())
                };
                unsafe { child_cmd.pre_exec(bind_stdin) };
            }
            FuzzInputSetup::SharedMemory(shm_id, shm_size) => {
                child_cmd.env("__AFL_SHM_FUZZ_ID", shm_id.to_string());
                child_cmd.env("__AFL_SHM_FUZZ_ID_SIZE", format!("{}", shm_size));
            }
            FuzzInputSetup::FileArg => {}
        }
    }
}

impl<SHM: ShMem> From<&FuzzInput<SHM>> for FuzzInputSetup {
    fn from(value: &FuzzInput<SHM>) -> Self {
        match value {
            FuzzInput::Stdin(file) => Self::Stdin(file.as_raw_fd()),
            FuzzInput::File(_) => Self::FileArg,
            FuzzInput::SharedMemory(ref shm) => Self::SharedMemory(shm.id(), shm.len()),
        }
    }
}

/// The [`Forkserver`] is communication channel with a child process that forks on request of the fuzzer.
/// The communication happens via pipe.
#[derive(Debug)]
pub struct NeoForkServer {
    /// The "actual" forkserver we spawned in the target
    fork_server_child: Child,
    /// Status pipe reader
    rx: PipeReader,
    /// Control pipe writer
    tx: PipeWriter,
    /// Pid of the current forked child (child of the forkserver) during execution
    child_pid: Option<Pid>,
    /// If the last run timed out (in in-target i32)
    last_run_timed_out: bool,
    /// The signal this [`Forkserver`] will use to kill (defaults to [`self.kill_signal`])
    kill_signal: Signal,
}

const FORKSRV_CTL_FD: i32 = 198;
const FORKSRV_ST_FD: i32 = FORKSRV_CTL_FD + 1;

impl Drop for NeoForkServer {
    fn drop(&mut self) {
        if let Some(pid) = self.child_pid {
            debug!("Sending {} to child {pid}", self.kill_signal);
            if let Err(err) = nix::sys::signal::kill(pid, self.kill_signal) {
                warn!(
                    "Failed to deliver kill signal to child process {}: {err} ({})",
                    pid,
                    io::Error::last_os_error()
                );
            }
        }

        if let Err(err) = self.fork_server_child.kill() {
            warn!(%err, "Fail to kill fork server process.");
        } else if let Err(err) = self.fork_server_child.wait() {
            warn!(%err, "Fail to wait fork server process.");
        }
    }
}

#[derive(Debug)]
pub struct NeoForkServerOptions {
    pub target: OsString,
    pub args: Vec<OsString>,
    pub envs: Vec<(OsString, OsString)>,
    pub input_setup: FuzzInputSetup,
    pub memlimit: u64,
    pub persistent_fuzzing: bool,
    pub deferred: bool,
    pub coverage_map_info: Option<(ShMemId, usize)>,
    pub afl_debug: bool,
    pub debug_output: bool,
    pub kill_signal: Signal,
}

impl NeoForkServer {
    pub fn new(options: NeoForkServerOptions) -> Result<Self, libafl::Error> {
        let NeoForkServerOptions {
            target,
            args,
            envs,
            input_setup,
            memlimit,
            persistent_fuzzing,
            deferred,
            coverage_map_info,
            afl_debug,
            debug_output,
            kill_signal,
        } = options;

        let (rx, child_writer) = os_pipe::pipe().afl_context("Fal to create ex pipe.")?;
        let (child_reader, tx) = os_pipe::pipe().afl_context("Fail to create tx pipe.")?;

        let (stdout, stderr) = if debug_output {
            (Stdio::inherit(), Stdio::inherit())
        } else {
            (Stdio::null(), Stdio::null())
        };

        let mut command = process::Command::new(target);
        // Setup args, stdio
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);

        if let Some((shm_id, map_size)) = coverage_map_info {
            command.env("__AFL_SHM_ID", shm_id.to_string());
            command.env("__AFL_SHM_ID_SIZE", format!("{}", map_size));
            command.env("AFL_MAP_SIZE", format!("{}", map_size));
        }

        persistent_fuzzing.then(|| command.env("__AFL_PERSISTENT", "1"));
        deferred.then(|| command.env("__AFL_DEFER_FORKSRV", "1"));

        command
            .env("LD_BIND_NOW", "1")
            .envs(envs)
            .setlimit(memlimit)
            .set_coredump(afl_debug)
            .setsid();

        let bind_pipes = move || {
            use nix::unistd::dup2;
            dup2(child_reader.as_raw_fd(), FORKSRV_CTL_FD)
                .map_err(|_| io::Error::last_os_error())?;
            dup2(child_writer.as_raw_fd(), FORKSRV_ST_FD)
                .map_err(|_| io::Error::last_os_error())?;
            Ok(())
        };
        unsafe { command.pre_exec(bind_pipes) };

        input_setup.setup_child_cmd(&mut command);
        let fork_server_child = command.spawn().map_err(|err| {
            libafl::Error::illegal_state(format!("Could not spawn the forkserver: {err:#?}"))
        })?;

        Ok(Self {
            fork_server_child,
            rx,
            tx,
            child_pid: None,
            last_run_timed_out: false,
            kill_signal,
        })
    }

    fn handshake(&mut self) -> Result<i32, libafl::Error> {
        let handshake_msg = self
            .read_i32()
            .afl_context("Oops the fork server fucked up.")?;
        check_handshake_error_bits(handshake_msg)?;
        check_version(handshake_msg)?;
        let handshake_response = handshake_msg as u32 ^ 0xffffffff;
        self.write_u32(handshake_response)
            .afl_context("Fail to write handshake response to forkserver")?;
        Ok(handshake_msg)
    }

    /// Initialize the fork server and return the options
    pub fn initialize(&mut self) -> Result<ForkServerOptions, libafl::Error> {
        let handshake_msg = self.handshake()?;
        let flags = self
            .read_i32()
            .afl_context("Fail to read option flags from fork server.")?;
        let flags = ForkServerFlags::from_bits(flags)
            .afl_context("Fail to parse option flags from fork server.")?;
        let map_size = flags
            .contains(ForkServerFlags::MAP_SIZE)
            .then(|| self.read_i32().map(|it| it as usize))
            .transpose()
            .afl_context("Fail to read map size from fork server.")?;
        let shmem_fuzz = flags.contains(ForkServerFlags::SHMEM_FUZZ);
        let autodict = flags
            .contains(ForkServerFlags::AUTODICT)
            .then(|| -> Result<_, libafl::Error> {
                let autotokens_size = self
                    .read_i32()
                    .afl_context("Fail to read autotokens size from fork server.")?;
                let tokens_size_max = 0xffffff;

                if !(2..=tokens_size_max).contains(&autotokens_size) {
                    let message = format!(
                        "Autotokens size is incorrect, expected 2 to {tokens_size_max} (inclusive), \
                            but got {autotokens_size}. Make sure your afl-cc version is up to date."
                    );
                    Err(libafl::Error::illegal_state(message))?;
                }
                info!(size = autotokens_size, "AUTODICT detected.");
                let autotokens = self
                    .read_vec(autotokens_size as usize)
                    .afl_context("Fail to read autotokens from fork server.")?;
                Ok(autotokens)
            })
            .transpose()?;
        let final_handshake_msg = self
            .read_i32()
            .afl_context("Fail to read final handshake information from fork server.")?;
        if final_handshake_msg != handshake_msg {
            Err(libafl::Error::unknown(
                "Final handshake message does not match",
            ))?;
        }
        Ok(ForkServerOptions {
            map_size,
            shmem_fuzz,
            autodict,
        })
    }

    pub fn run_child(&mut self, timeout: &TimeSpec) -> Result<(Pid, Option<i32>), libafl::Error> {
        let notification = u32::from(self.last_run_timed_out);
        self.write_u32(notification)
            .afl_context("Oops the fork server is dead.")?;

        let child_pid = self
            .read_i32()
            .afl_context("Fail to get child PID from fork server")?;
        if child_pid <= 0 {
            Err(libafl::Error::unknown(
                "Get an invalid PID from fork server.",
            ))?;
        }
        let pid = Pid::from_raw(child_pid);
        self.child_pid = Some(pid);

        let status = self.read_st_timed(timeout)?;
        self.last_run_timed_out = status.is_none();

        if self.last_run_timed_out {
            match nix::sys::signal::kill(pid, self.kill_signal) {
                Ok(_) | Err(Errno::ESRCH) => {
                    // It is OK if the child terminated before we could kill it
                }
                Err(errno) => {
                    let message =
                        format!("Oops we could not kill timed-out child: {}", errno.desc());
                    Err(libafl::Error::unknown(message))?;
                }
            }
            self.read_u32()
                .afl_context("Could not kill time-out child")?;
        }

        if status.is_some_and(|it| !libc::WIFSTOPPED(it)) {
            self.child_pid = None;
        }

        Ok((pid, status))
    }

    /// Read from the st pipe
    fn read_u32(&mut self) -> Result<u32, libafl::Error> {
        let mut buf: [u8; 4] = [0_u8; 4];
        self.rx.read_exact(&mut buf)?;
        Ok(u32::from_ne_bytes(buf))
    }

    /// Read from the st pipe
    fn read_i32(&mut self) -> Result<i32, libafl::Error> {
        let mut buf: [u8; 4] = [0_u8; 4];
        self.rx.read_exact(&mut buf)?;
        Ok(i32::from_ne_bytes(buf))
    }

    /// Read bytes of any length from the st pipe
    fn read_vec(&mut self, size: usize) -> Result<Vec<u8>, libafl::Error> {
        let mut buf = Vec::with_capacity(size);
        unsafe {
            // SAFETY: `buf` is guaranteed to have a capacity of `size` bytes.
            //         Therefore the `slice` will not reach non-accessible memory.
            let slice = slice::from_raw_parts_mut(buf.as_mut_ptr(), size);
            self.rx.read_exact(slice)?;
            // SAFETY: `buf` must have been filled with `size` bytes upon this point.
            buf.set_len(size);
        };
        Ok(buf)
    }

    /// Write to the ctl pipe
    fn write_u32(&mut self, val: u32) -> Result<(), libafl::Error> {
        self.tx.write_all(&val.to_ne_bytes())?;
        Ok(())
    }

    /// Read a message from the child process.
    fn read_st_timed(&mut self, timeout: &TimeSpec) -> Result<Option<i32>, libafl::Error> {
        let st_read = self.rx.as_raw_fd();

        // # Safety
        // The FDs are valid as this point in time.
        let st_read = unsafe { BorrowedFd::borrow_raw(st_read) };

        let mut readfds = FdSet::new();
        readfds.insert(st_read);
        // We'll pass a copied timeout to keep the original timeout intact,
        // because select updates timeout to indicate how much time was left. See select(2)
        let sret = nix::sys::select::pselect(
            None,
            &mut readfds,
            None,
            None,
            Some(timeout),
            Some(&SigSet::empty()),
        )?;
        if sret > 0 {
            let mut buf: [u8; 4] = [0_u8; 4];
            self.rx
                .read_exact(&mut buf)
                .map(move |()| Some(i32::from_ne_bytes(buf)))
                .map_err(|_| {
                    libafl::Error::unknown("Unable to communicate with fork server (OOM?)")
                })
        } else {
            Ok(None)
        }
    }
}

const FSRV_NEW_VERSION_MIN: u32 = 1;
const FSRV_NEW_VERSION_MAX: u32 = 1;

pub(super) fn check_version(handshake_msg: i32) -> Result<(), libafl::Error> {
    reject_old_forkserver(handshake_msg)?;

    let version: u32 = handshake_msg as u32 - 0x41464c00_u32;
    match version {
        FSRV_NEW_VERSION_MIN..=FSRV_NEW_VERSION_MAX => Ok(()),
        0 => Err(libafl::Error::unknown(
            "Fork server version is not assigned, this should not happen. Recompile target.",
        ))?,
        _ => Err(libafl::Error::unknown(
            "Fork server version is not supported. Recompile the target.",
        ))?,
    }
}

fn reject_old_forkserver(handshake_msg: i32) -> Result<(), libafl::Error> {
    match handshake_msg {
        0x41464c00..0x41464cff => Ok(()),
        _ => Err(libafl::Error::unknown(
            "Old fork server model is used by the target, it is nolonger supported",
        )),
    }
}

pub(super) fn check_handshake_error_bits(handshake_msg: i32) -> Result<(), libafl::Error> {
    #[allow(clippy::cast_possible_wrap)]
    const FS_NEW_ERROR: i32 = 0xeffe0000_u32 as _;

    const FS_ERROR_MAP_SIZE: i32 = 1 << 0;
    const FS_ERROR_MAP_ADDR: i32 = 1 << 1;
    const FS_ERROR_SHM_OPEN: i32 = 1 << 2;
    const FS_ERROR_SHMAT: i32 = 1 << 3;
    const FS_ERROR_MMAP: i32 = 1 << 4;
    const FS_ERROR_OLD_CMPLOG: i32 = 1 << 5;
    const FS_ERROR_OLD_CMPLOG_QEMU: i32 = 1 << 6;

    if (handshake_msg & FS_NEW_ERROR) == FS_NEW_ERROR {
        let error_code = handshake_msg & 0x0000ffff;
        let err = match error_code {
            FS_ERROR_MAP_SIZE =>  libafl::Error::unknown(
                "AFL_MAP_SIZE is not set and fuzzing target reports that the required size is very large. \
                      Solution: Run the fuzzing target stand-alone with the environment variable AFL_DEBUG=1 set \
                      and set the value for __afl_final_loc in the AFL_MAP_SIZE environment variable for afl-fuzz."
            ) ,
            FS_ERROR_MAP_ADDR => libafl::Error::unknown(
                "The fuzzing target reports that hardcoded map address might be the reason the mmap of the \
                  shared memory failed. Solution: recompile the target with either afl-clang-lto and do not \
                  set AFL_LLVM_MAP_ADDR or recompile with afl-clang-fast."
            ),
            FS_ERROR_SHM_OPEN => libafl::Error::unknown(
                "The fuzzing target reports that the shm_open() call failed."
            ),
            FS_ERROR_SHMAT => libafl::Error::unknown("The fuzzing target reports that the shmat() call failed."),
            FS_ERROR_MMAP => libafl::Error::unknown(
                "The fuzzing target reports that the mmap() call to the shared memory failed."
            ),
            FS_ERROR_OLD_CMPLOG => libafl::Error::unknown(
                "The -c cmplog target was instrumented with an too old AFL++ version, you need to recompile it."
            ),
            FS_ERROR_OLD_CMPLOG_QEMU => libafl::Error::unknown(
                "The AFL++ QEMU/FRIDA loaders are from an older version, for -c you need to recompile it."
            ),
            code => libafl::Error::unknown(format!("Unknown error code {code} from fuzzing target!")),
        };
        Err(err)
    } else {
        Ok(())
    }
}
