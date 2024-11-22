use std::{
    fs::File,
    io::{self},
    path::Path,
};

use memmap2::MmapOptions;

use crate::afl;

#[derive(Debug)]
pub struct FuzzBinaryInfo {
    pub is_afl_instrumented: bool,
    pub is_persistent_mode: bool,
    pub is_defer_fork_server: bool,
    pub uses_address_sanitizer: bool,
}

const ASAN_INIT_SIGNATURE: &[u8] = b"__asan_init";
const PERSISTENT_MODE_SIGNATURE: &[u8] = b"##SIG_AFL_PERSISTENT##\0";
const DEFER_FORK_SERVER_SIGNATURE: &[u8] = b"##SIG_AFL_DEFER_FORKSRV##\0";

impl FuzzBinaryInfo {
    pub fn from_binary(path: &Path) -> io::Result<Self> {
        let binary_file = File::open(path)?;
        // SAFETY: We are assuming that the file is not touched externally
        let file_slice = unsafe { MmapOptions::new().map(&binary_file) }?;
        let is_afl_instrumented =
            kmp::kmp_find(afl::SHMEM_ADDR_ENV.as_bytes(), &file_slice).is_some();
        let is_persistent_mode = kmp::kmp_find(PERSISTENT_MODE_SIGNATURE, &file_slice).is_some();
        let is_defer_fork_server =
            kmp::kmp_find(DEFER_FORK_SERVER_SIGNATURE, &file_slice).is_some();
        let uses_address_sanitizer = kmp::kmp_find(ASAN_INIT_SIGNATURE, &file_slice).is_some();
        Ok(Self {
            is_afl_instrumented,
            is_persistent_mode,
            is_defer_fork_server,
            uses_address_sanitizer,
        })
    }
}
