use std::{
    io::{self, Read},
    path::Path,
    process::Stdio,
};

use anyhow::Context;

use crate::afl;

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
