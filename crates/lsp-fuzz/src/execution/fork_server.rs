//! Implementation of a fork server for efficient process creation during fuzzing.
//!
//! This module provides an implementation of a fork server that communicates with a child process
//! which forks on request from the fuzzer. This approach is more efficient than starting a new
//! process for each fuzzing iteration.

use std::{
    ffi::OsString,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, BorrowedFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{self, Child, Command, Stdio},
};

use bitflags::bitflags;
use libafl::executors::forkserver::ConfigTarget;
use libafl_bolts::{
    fs::InputFile,
    shmem::{ShMem, ShMemId},
};
use nix::{
    errno::Errno,
    sys::{
        select::FdSet,
        signal::{SigSet, Signal},
        time::TimeSpec,
        wait::{WaitPidFlag, WaitStatus},
    },
    unistd::Pid,
};
use os_pipe::{PipeReader, PipeWriter};
use tracing::{debug, info, warn};

use super::FuzzInput;
use crate::utils::AflContext;

/// Represents the different ways to set up fuzzing input for the target.
#[derive(Debug)]
pub enum FuzzInputSetup<'f> {
    /// Input is passed via stdin to the target.
    Stdin(&'f InputFile),
    /// Input is passed as a file argument to the target.
    FileArg,
    /// Input is passed through shared memory to the target.
    SharedMemory(ShMemId, usize),
}

// Fork server option flags
mod new_fs_options {
    pub const MAPSIZE: i32 = 1 << 0;
    pub const SHDMEM_FUZZ: i32 = 1 << 1;
    pub const AUTODICT: i32 = 1 << 11;
}

bitflags! {
    struct ForkServerFlags: i32 {
        const MAP_SIZE = new_fs_options::MAPSIZE;
        const SHMEM_FUZZ = new_fs_options::SHDMEM_FUZZ;
        const AUTODICT = new_fs_options::AUTODICT;
    }
}

/// Contains information about the target provided by the fork server after initialization.
#[derive(Debug)]
pub struct ForkServerTargetInfo {
    /// Optional coverage map size reported by the target.
    pub map_size: Option<usize>,
    /// Whether shared memory fuzzing is supported.
    pub shmem_fuzz: bool,
    /// Optional dictionary of tokens automatically extracted from the target.
    pub autodict: Option<Vec<u8>>,
}

impl FuzzInputSetup<'_> {
    /// Configures the command for the child process based on the input setup.
    ///
    /// This function sets up stdin, environment variables, or other command settings
    /// needed for the specific input type.
    pub fn setup_child_cmd(&self, child_cmd: &mut Command) {
        match self {
            &FuzzInputSetup::Stdin(input_file) => {
                let stdin = unsafe {
                    // SAFETY: The file descriptor is reliable and valid because it comes from InputFile.
                    //         The file reference being alive means we have an accessible open file.
                    Stdio::from_raw_fd(input_file.as_raw_fd())
                };
                child_cmd.stdin(stdin);
            }
            FuzzInputSetup::SharedMemory(shm_id, shm_size) => {
                child_cmd.env("__AFL_SHM_FUZZ_ID", shm_id.to_string());
                child_cmd.env("__AFL_SHM_FUZZ_ID_SIZE", shm_size.to_string());
            }
            // FileArg doesn't need special setup as the file path will be passed as an argument
            FuzzInputSetup::FileArg => {}
        }
    }
}

/// Convert from a FuzzInput to a FuzzInputSetup
impl<'a, SHM: ShMem> From<&'a FuzzInput<SHM>> for FuzzInputSetup<'a> {
    fn from(value: &'a FuzzInput<SHM>) -> Self {
        match value {
            FuzzInput::Stdin(file) => Self::Stdin(file),
            FuzzInput::File(_) => Self::FileArg,
            FuzzInput::SharedMemory(shm) => Self::SharedMemory(shm.id(), shm.len()),
        }
    }
}

// File descriptor constants used for fork server communication
mod fd {
    /// Control file descriptor - used to send commands to the fork server
    pub const CONTROL: i32 = 198;
    /// Status file descriptor - used to receive status from the fork server
    pub const STATUS: i32 = CONTROL + 1;
}

/// The [`NeoForkServer`] is a communication channel with a child process that forks on request of the fuzzer.
///
/// This implementation uses pipes for bidirectional communication with the forked process.
/// It handles spawning the initial fork server process, setting up the communication channels,
/// and managing the lifecycle of child processes during fuzzing.
#[derive(Debug)]
pub struct NeoForkServer {
    /// The persistent fork server process we initially spawn
    fork_server_child: Child,
    /// Status pipe reader - receives messages from the fork server
    rx: PipeReader,
    /// Control pipe writer - sends commands to the fork server
    tx: PipeWriter,
    /// PID of the current active child (child of the fork server) during execution
    child_pid: Option<Pid>,
    /// Tracks whether the last run timed out
    last_run_timed_out: bool,
    /// The signal to use when killing child processes that time out
    kill_signal: Signal,
}

impl Drop for NeoForkServer {
    fn drop(&mut self) {
        // First, try to kill any active child process (if one exists)
        if let Some(pid) = self.child_pid {
            debug!("Sending {} to child {pid}", self.kill_signal);
            match nix::sys::signal::kill(pid, self.kill_signal) {
                // Success or process already gone (ESRCH) are both fine
                Ok(()) | Err(Errno::ESRCH) => (),
                // Log other errors but continue with cleanup
                Err(err) => {
                    warn!(
                        "Failed to deliver kill signal to child process {}: {err} ({})",
                        pid,
                        io::Error::last_os_error()
                    );
                }
            }
        }

        // Then, clean up the fork server process
        if let Err(err) = self.fork_server_child.kill() {
            warn!(%err, "Failed to kill fork server process");
        } else if let Err(err) = self.fork_server_child.wait() {
            warn!(%err, "Failed to wait for fork server process");
        }
    }
}

/// Configuration options for creating a new fork server.
#[derive(Debug)]
pub struct NeoForkServerOptions<'a> {
    /// Path to the target executable
    pub target: OsString,
    /// Command-line arguments to pass to the target
    pub args: Vec<OsString>,
    /// Environment variables to set for the target
    pub envs: Vec<(OsString, OsString)>,
    /// Configuration for how fuzzing input is provided to the target
    pub input_setup: FuzzInputSetup<'a>,
    /// Memory limit (in bytes) for the target process
    pub memlimit: u64,
    /// Whether to use persistent fuzzing mode (target runs multiple test cases without restarting)
    pub persistent_fuzzing: bool,
    /// Whether to use deferred fork server initialization
    pub deferred: bool,
    /// Optional shared memory ID and size for coverage map
    pub coverage_map_info: (ShMemId, usize),
    /// Whether to enable AFL debug mode
    pub afl_debug: bool,
    /// Whether to show stdout/stderr from the target
    pub debug_output: bool,
    /// Signal to use when killing child processes
    pub kill_signal: Signal,
    pub stdout_capture_fd: BorrowedFd<'a>,
}

impl NeoForkServer {
    /// Creates a new fork server with the given options.
    ///
    /// This method:
    /// 1. Sets up communication pipes with the target
    /// 2. Configures the target process with environment variables and limits
    /// 3. Spawns the initial fork server process
    pub fn new(options: NeoForkServerOptions<'_>) -> Result<Self, libafl::Error> {
        let NeoForkServerOptions {
            target,
            args,
            envs,
            input_setup,
            memlimit,
            persistent_fuzzing,
            deferred,
            coverage_map_info: (shm_id, map_size),
            afl_debug,
            debug_output,
            kill_signal,
            stdout_capture_fd,
        } = options;

        // Create bidirectional pipes for communication with the fork server
        let (rx, child_writer) = os_pipe::pipe().afl_context("Failed to create status pipe")?;
        let (child_reader, tx) = os_pipe::pipe().afl_context("Failed to create control pipe")?;

        // Configure stdio based on debug settings
        let stderr = debug_output
            .then(Stdio::inherit)
            .unwrap_or_else(Stdio::null);

        // Create and configure the command
        let mut command = process::Command::new(target);
        command
            .args(args)
            .stdin(Stdio::null()) // Will be overridden by input_setup if necessary
            .stderr(stderr);

        command.env("__AFL_SHM_ID", shm_id.to_string());
        command.env("__AFL_SHM_ID_SIZE", map_size.to_string());
        command.env("AFL_MAP_SIZE", map_size.to_string());

        // Configure debug and fuzzing options
        if debug_output {
            command.env("AFL_DEBUG_CHILD", "1");
        }

        if afl_debug {
            command.env("AFL_DEBUG", "1");
        }

        if persistent_fuzzing {
            command.env("__AFL_PERSISTENT", "1");
        }

        if deferred {
            command.env("__AFL_DEFER_FORKSRV", "1");
        }

        // Set additional environment variables and process limits
        command
            .env("LD_BIND_NOW", "1")
            .envs(envs)
            .setlimit(memlimit)
            .set_coredump(afl_debug)
            .setsid();

        // Set up the pipe file descriptors in the child process
        let bind_pipes = {
            let child_reader_fd = child_reader.as_raw_fd();
            let child_writer_fd = child_writer.as_raw_fd();
            let output_capture_fd = stdout_capture_fd.as_raw_fd();
            let communication_fds = [
                rx.as_raw_fd(),
                tx.as_raw_fd(),
                child_writer_fd,
                child_reader_fd,
            ];
            move || {
                use nix::unistd::{close, dup2};
                dup2(output_capture_fd, nix::libc::STDOUT_FILENO).map_err(io::Error::from)?;
                dup2(child_reader_fd, fd::CONTROL).map_err(io::Error::from)?;
                dup2(child_writer_fd, fd::STATUS).map_err(io::Error::from)?;
                for fd in communication_fds {
                    close(fd).map_err(io::Error::from)?;
                }
                Ok(())
            }
        };
        unsafe { command.pre_exec(bind_pipes) };

        // Increase stack size to avoid stack overflows due to address sanitizer
        let increase_stack_size = || {
            use nix::sys::resource::{Resource, setrlimit};
            const STACK_SIZE: libc::rlim_t = 0x1E00000;
            setrlimit(Resource::RLIMIT_STACK, STACK_SIZE, STACK_SIZE).map_err(io::Error::from)
        };
        unsafe { command.pre_exec(increase_stack_size) };

        // Set up input method (stdin, file, or shared memory)
        input_setup.setup_child_cmd(&mut command);

        // Spawn the fork server process
        let fork_server_child = command.spawn().map_err(|err| {
            libafl::Error::illegal_state(format!("Could not spawn the fork server: {err:#?}"))
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

    /// Perform initial handshake with the fork server.
    ///
    /// This exchanges protocol version information and validates compatibility.
    fn handshake(&mut self) -> Result<i32, libafl::Error> {
        let handshake_msg = self
            .read_i32()
            .afl_context("Fork server handshake failed")?;

        // Check for errors and verify protocol version
        check_handshake_error_bits(handshake_msg)?;
        check_version(handshake_msg)?;

        // Compute and send the handshake response (inverted message)
        let handshake_response = handshake_msg as u32 ^ 0xffffffff;
        self.write_u32(handshake_response)
            .afl_context("Failed to write handshake response to fork server")?;

        Ok(handshake_msg)
    }

    /// Initialize the fork server and return the target information.
    ///
    /// This method:
    /// 1. Performs the initial handshake
    /// 2. Reads capability flags from the fork server
    /// 3. Retrieves target-specific information like map size and dictionary
    pub fn initialize(&mut self) -> Result<ForkServerTargetInfo, libafl::Error> {
        // Perform initial handshake
        let handshake_msg = self.handshake().afl_context("Handshake failed")?;

        // Read and parse capability flags
        let flags = self
            .read_i32()
            .afl_context("Failed to read option flags from fork server")?;
        let flags = ForkServerFlags::from_bits(flags)
            .afl_context("Fail to parse option flags from fork server.")?;
        let map_size = flags
            .contains(ForkServerFlags::MAP_SIZE)
            .then(|| self.read_i32().map(|it| it as usize))
            .transpose()
            .afl_context("Fail to read map size from fork server.")?;
        let shmem_fuzz = flags.contains(ForkServerFlags::SHMEM_FUZZ);

        // Read optional dictionary if supported
        let autodict = if flags.contains(ForkServerFlags::AUTODICT) {
            // Read dictionary size
            let autotokens_size = self
                .read_i32()
                .afl_context("Failed to read autodict size from fork server")?;
            let tokens_size_max = 0xffffff;

            // Validate dictionary size
            if !(2..=tokens_size_max).contains(&autotokens_size) {
                let message = format!(
                    "Autodict size is incorrect, expected 2 to {tokens_size_max} (inclusive), \
                    but got {autotokens_size}. Make sure your afl-cc version is up to date."
                );
                return Err(libafl::Error::illegal_state(message));
            }

            info!(size = autotokens_size, "AUTODICT detected");

            // Read the dictionary data
            let autotokens = self
                .read_vec(autotokens_size as usize)
                .afl_context("Failed to read autodict data from fork server")?;

            Some(autotokens)
        } else {
            None
        };

        // Verify final handshake message matches initial one
        let final_handshake_msg = self
            .read_i32()
            .afl_context("Failed to read final handshake message from fork server")?;

        if final_handshake_msg != handshake_msg {
            return Err(libafl::Error::unknown(
                "Final handshake message does not match initial message",
            ));
        }

        // Return the target information
        Ok(ForkServerTargetInfo {
            map_size,
            shmem_fuzz,
            autodict,
        })
    }

    /// Run a child process through the fork server with a timeout.
    ///
    /// Returns the process ID and exit status (if the process completed within timeout).
    pub fn run_child(&mut self, timeout: &TimeSpec) -> Result<(Pid, Option<i32>), libafl::Error> {
        while nix::sys::wait::waitpid(None, Some(WaitPidFlag::WNOHANG))
            .afl_context("Waiting for child processes")?
            != WaitStatus::StillAlive
        {}

        // Notify fork server if the previous run timed out
        let notification = u32::from(self.last_run_timed_out);
        self.write_u32(notification)
            .afl_context("Fork server communication failed")?;

        // Get the child process ID
        let child_pid = self
            .read_i32()
            .afl_context("Failed to get child PID from fork server")?;

        if child_pid <= 0 {
            return Err(libafl::Error::unknown(
                "Invalid PID received from fork server",
            ));
        }

        let pid = Pid::from_raw(child_pid);
        self.child_pid = Some(pid);

        // Wait for the child to complete, with timeout
        let status = self.read_st_timed(timeout)?;
        self.last_run_timed_out = status.is_none();

        // Handle timeout case
        if self.last_run_timed_out {
            // Try to kill the child process
            match nix::sys::signal::kill(pid, self.kill_signal) {
                Ok(_) | Err(Errno::ESRCH) => {
                    // It's OK if the child already terminated
                }
                Err(errno) => {
                    let message =
                        format!("Failed to kill timed-out child process: {}", errno.desc());
                    return Err(libafl::Error::unknown(message));
                }
            }

            // Read acknowledgment from fork server
            self.read_u32()
                .afl_context("Failed to get kill acknowledgment from fork server")?;
        }

        // Clear child_pid if process completed (not just stopped)
        if status.is_some_and(|it| !libc::WIFSTOPPED(it)) {
            self.child_pid = None;
        }

        Ok((pid, status))
    }

    /// Read a 32-bit unsigned integer from the status pipe.
    fn read_u32(&mut self) -> Result<u32, libafl::Error> {
        let mut buf: [u8; 4] = [0_u8; 4];
        self.rx.read_exact(&mut buf)?;
        Ok(u32::from_ne_bytes(buf))
    }

    /// Read a 32-bit signed integer from the status pipe.
    fn read_i32(&mut self) -> Result<i32, libafl::Error> {
        let mut buf: [u8; 4] = [0_u8; 4];
        self.rx.read_exact(&mut buf)?;
        Ok(i32::from_ne_bytes(buf))
    }

    /// Read a vector of bytes from the status pipe.
    fn read_vec(&mut self, size: usize) -> Result<Vec<u8>, libafl::Error> {
        let mut buf = Vec::with_capacity(size);
        unsafe {
            // SAFETY: We just allocated enough space for the buffer
            //         and we will not return `buf` unless it is fully filled.
            buf.set_len(size);
            self.rx.read_exact(&mut buf)
        }?;
        Ok(buf)
    }

    /// Write a 32-bit unsigned integer to the control pipe.
    fn write_u32(&mut self, val: u32) -> Result<(), libafl::Error> {
        self.tx.write_all(&val.to_ne_bytes())?;
        Ok(())
    }

    /// Read a message from the status pipe with a timeout.
    ///
    /// Returns Some(status) if data is received within the timeout period,
    /// or None if the timeout expires.
    fn read_st_timed(&mut self, timeout: &TimeSpec) -> Result<Option<i32>, libafl::Error> {
        let st_read = self.rx.as_raw_fd();

        // SAFETY: The file descriptor is valid at this point
        let st_read = unsafe { BorrowedFd::borrow_raw(st_read) };

        // Set up the file descriptor set for select
        let mut readfds = FdSet::new();
        readfds.insert(st_read);

        // Set up signal mask to allow interruption by SIGINT
        let mut sigset = SigSet::empty();
        sigset.add(Signal::SIGINT);

        // Wait for data with timeout
        let sret = nix::sys::select::pselect(
            None,
            &mut readfds,
            None,
            None,
            Some(timeout),
            Some(&sigset),
        )?;

        if sret > 0 {
            // Data is available, read it
            let mut buf: [u8; 4] = [0_u8; 4];
            self.rx
                .read_exact(&mut buf)
                .map(move |()| Some(i32::from_ne_bytes(buf)))
                .map_err(|_| {
                    libafl::Error::unknown("Unable to communicate with fork server (OOM?)")
                })
        } else {
            // Timeout or no data available
            Ok(None)
        }
    }
}

// Version constants
mod version {
    /// Minimum supported fork server protocol version
    pub const MIN: u32 = 1;
    /// Maximum supported fork server protocol version
    pub const MAX: u32 = 1;
    /// AFL protocol magic number base
    pub const AFL_MAGIC_BASE: u32 = 0x41464c00;
}

// Error codes reported by the fork server
mod fs_error {
    /// Error flag in handshake message
    #[allow(clippy::cast_possible_wrap)]
    pub const ERROR_FLAG: i32 = 0xeffe0000_u32 as i32;

    // Specific error codes
    pub const MAP_SIZE: i32 = 1 << 0;
    pub const MAP_ADDR: i32 = 1 << 1;
    pub const SHM_OPEN: i32 = 1 << 2;
    pub const SHMAT: i32 = 1 << 3;
    pub const MMAP: i32 = 1 << 4;
    pub const OLD_CMPLOG: i32 = 1 << 5;
    pub const OLD_CMPLOG_QEMU: i32 = 1 << 6;
}

/// Checks if the fork server version is supported.
///
/// This verifies:
/// 1. The message contains a valid AFL magic number
/// 2. The protocol version is within supported range
pub(super) fn check_version(handshake_msg: i32) -> Result<(), libafl::Error> {
    // Check for valid AFL magic number range
    if !(version::AFL_MAGIC_BASE as i32 <= handshake_msg
        && handshake_msg <= (version::AFL_MAGIC_BASE + 0xff) as i32)
    {
        return Err(libafl::Error::unknown(
            "Old fork server model is used by the target, it is no longer supported",
        ));
    }

    // Extract version from the handshake message
    let version = (handshake_msg as u32) - version::AFL_MAGIC_BASE;

    match version {
        0 => Err(libafl::Error::unknown(
            "Fork server version is not assigned. This should not happen. Recompile target.",
        )),
        v if version::MIN <= v && v <= version::MAX => Ok(()),
        _ => Err(libafl::Error::unknown(
            "Unsupported fork server version. Recompile the target with a compatible AFL version.",
        )),
    }
}

/// Checks if the handshake message contains error flags.
///
/// If errors are detected, returns a user-friendly error message
/// with guidance on how to fix the issue.
pub(super) fn check_handshake_error_bits(handshake_msg: i32) -> Result<(), libafl::Error> {
    // Check if error flag is set
    if (handshake_msg & fs_error::ERROR_FLAG) == fs_error::ERROR_FLAG {
        // Extract the specific error code
        let error_code = handshake_msg & 0x0000ffff;

        // Map error code to a specific error message
        let error_message = match error_code {
            fs_error::MAP_SIZE => {
                "AFL_MAP_SIZE is not set and the target reports that the required size is very large. \
                Solution: Run the target with AFL_DEBUG=1 and set the value for __afl_final_loc \
                in the AFL_MAP_SIZE environment variable for afl-fuzz."
            }

            fs_error::MAP_ADDR => {
                "The target reports that a hardcoded map address might be causing the shared memory \
                mapping to fail. Solution: Recompile with afl-clang-lto without setting \
                AFL_LLVM_MAP_ADDR, or use afl-clang-fast instead."
            }

            fs_error::SHM_OPEN => "The target reports that the shm_open() call failed.",

            fs_error::SHMAT => "The target reports that the shmat() call failed.",

            fs_error::MMAP => {
                "The target reports that the mmap() call to the shared memory failed."
            }

            fs_error::OLD_CMPLOG => {
                "The -c cmplog target was instrumented with an outdated AFL++ version. \
                You need to recompile it with a newer version."
            }

            fs_error::OLD_CMPLOG_QEMU => {
                "The AFL++ QEMU/FRIDA loaders are from an older version. \
                For -c support you need to recompile them."
            }

            code => &*format!("Unknown error code {code} from fuzzing target!"),
        };

        Err(libafl::Error::unknown(error_message))
    } else {
        Ok(())
    }
}
