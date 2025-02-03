use std::{
    fs::File,
    io::{self},
    path::Path,
};

use anyhow::{bail, Context};
use libafl_bolts::shmem::{ShMem, ShMemId, ShMemProvider};
use memmap2::Mmap;

use crate::{
    afl,
    execution::fork_server::{FuzzInputSetup, NeoForkServer, NeoForkServerOptions},
};

#[derive(Debug)]
pub struct StaticTargetBinaryInfo {
    pub is_afl_instrumented: bool,
    pub is_persistent_mode: bool,
    pub is_defer_fork_server: bool,
    pub uses_address_sanitizer: bool,
}

const ASAN_SIGNATURE: &[u8] = b"__asan_";
const PERSISTENT_MODE_SIGNATURE: &[u8] = b"##SIG_AFL_PERSISTENT##";
const DEFER_FORK_SERVER_SIGNATURE: &[u8] = b"##SIG_AFL_DEFER_FORKSRV##";

impl StaticTargetBinaryInfo {
    pub fn scan(binary: &Path) -> io::Result<Self> {
        let binary_file = File::open(binary)?;
        // SAFETY: We are assuming that the file is not touched externally
        let file_slice = unsafe { Mmap::map(&binary_file) }?;
        let is_afl_instrumented =
            kmp::kmp_find(afl::SHMEM_ADDR_ENV.as_bytes(), &file_slice).is_some();
        let is_persistent_mode = kmp::kmp_find(PERSISTENT_MODE_SIGNATURE, &file_slice).is_some();
        let is_defer_fork_server =
            kmp::kmp_find(DEFER_FORK_SERVER_SIGNATURE, &file_slice).is_some();
        let uses_address_sanitizer = kmp::kmp_find(ASAN_SIGNATURE, &file_slice).is_some();
        Ok(Self {
            is_afl_instrumented,
            is_persistent_mode,
            is_defer_fork_server,
            uses_address_sanitizer,
        })
    }
}

#[derive(Debug)]
pub struct TargetBinaryInfo {
    pub map_size: Option<usize>,
    pub is_shmem_fuzzing: bool,
    pub is_persistent_mode: bool,
    pub is_defer_fork_server: bool,
    pub uses_address_sanitizer: bool,
}

impl TargetBinaryInfo {
    pub fn detect(
        binary: &Path,
        shmem_provider: &mut impl ShMemProvider,
        static_info: StaticTargetBinaryInfo,
        afl_debug: bool,
        debug_child_output: bool,
    ) -> Result<Self, anyhow::Error> {
        if !static_info.is_afl_instrumented {
            bail!("Target is not instruemented by AFL++");
        }
        const MOCK_SHMEM_SIZE: usize = 65536;
        let mock_shmem = shmem_provider
            .new_shmem(MOCK_SHMEM_SIZE)
            .context("Creating mock shmem")?;
        let opts = NeoForkServerOptions {
            target: binary.into(),
            args: Vec::default(),
            envs: Vec::default(),
            input_setup: FuzzInputSetup::SharedMemory(ShMemId::default(), 0),
            memlimit: 0,
            persistent_fuzzing: static_info.is_persistent_mode,
            deferred: static_info.is_defer_fork_server,
            coverage_map_info: Some((mock_shmem.id(), MOCK_SHMEM_SIZE)),
            afl_debug,
            debug_output: debug_child_output,
            kill_signal: nix::sys::signal::Signal::SIGKILL,
        };
        let mut fork_server = NeoForkServer::new(opts).context("Creating fork server")?;
        let opts = fork_server
            .initialize()
            .context("Initializing fork server")?;
        Ok(Self {
            map_size: opts.map_size,
            is_shmem_fuzzing: opts.shmem_fuzz,
            is_persistent_mode: static_info.is_persistent_mode,
            is_defer_fork_server: static_info.is_defer_fork_server,
            uses_address_sanitizer: static_info.uses_address_sanitizer,
        })
    }
}
