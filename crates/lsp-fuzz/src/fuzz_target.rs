use std::{
    io::{self, Read},
    path::Path,
    process::Stdio,
};

use anyhow::Context;

use crate::afl;

#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "This public analysis result intentionally exposes independent binary feature flags."
)]
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
    /// Scans a target binary image for AFL++ and sanitizer signatures.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if reading the binary data source failed before calling this
    /// function. This implementation currently does not produce its own error values.
    pub fn scan(binary_file: &[u8]) -> io::Result<Self> {
        let is_afl_instrumented =
            kmp::kmp_find(afl::SHMEM_ADDR_ENV.as_bytes(), binary_file).is_some();
        let is_persistent_mode = kmp::kmp_find(PERSISTENT_MODE_SIGNATURE, binary_file).is_some();
        let is_defer_fork_server =
            kmp::kmp_find(DEFER_FORK_SERVER_SIGNATURE, binary_file).is_some();
        let uses_address_sanitizer = kmp::kmp_find(ASAN_SIGNATURE, binary_file).is_some();
        Ok(Self {
            is_afl_instrumented,
            is_persistent_mode,
            is_defer_fork_server,
            uses_address_sanitizer,
        })
    }
}

/// Runs the target binary with `AFL_DUMP_MAP_SIZE=1` and parses the reported map size.
///
/// # Errors
///
/// Returns an error if spawning the target fails, reading its stdout fails, or the emitted map
/// size cannot be parsed as a `usize`.
///
/// # Panics
///
/// Panics if the spawned child process does not expose a piped stdout after this function
/// explicitly requested one.
pub fn dump_map_size(binary: &Path) -> Result<usize, anyhow::Error> {
    let mut cmd = std::process::Command::new(binary);
    let mut child = cmd
        .env("AFL_DUMP_MAP_SIZE", "1")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdout = child.stdout.take().expect("We set it to pipe");
    let mut buf = Vec::new();
    stdout.read_to_end(&mut buf)?;
    let output = String::from_utf8_lossy(&buf);
    let map_size = output
        .trim()
        .parse()
        .with_context(|| format!("Fail to parse size from: \"{}\"", output.trim()))?;
    child.wait()?;
    Ok(map_size)
}
