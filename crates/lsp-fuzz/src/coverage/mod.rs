use std::{
    ffi::OsStr,
    fs::{self},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context;
use derive_new::new as New;
use tempfile::TempDir;

#[derive(Debug, New)]
pub struct CoverageDataGenerator {
    executable: PathBuf,
    args: Vec<String>,
}

impl CoverageDataGenerator {
    pub fn merge_llvm_raw_prof_data<I>(&self, inputs: I, merged_file: &Path) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        Command::new("llvm-profdata")
            .args(["merge", "-sparse"])
            .arg("-o")
            .arg(merged_file)
            .args(inputs.into_iter())
            .status()
            .context("Running llvm-profdata")?;
        Ok(())
    }

    pub fn run_target_with_coverage(
        &self,
        input_bytes: &[u8],
        llvm_profile_raw: &OsStr,
    ) -> Result<(), anyhow::Error> {
        let working_dir = TempDir::new().context("Creating temp working dir")?;
        let mut process = Command::new(&self.executable)
            .args(&self.args)
            .env("LLVM_PROFILE_FILE", llvm_profile_raw)
            // We must take the reference here
            // Otherwirse it gets dropped inside and the directory is deleted.
            .current_dir(&working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Spawning target")?;
        let mut stdin = process.stdin.take().expect("We set it to pipe");
        match stdin.write_all(input_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                // This probably means that the target is dead.
            }
            err => err.context("Writing to target stdin")?,
        }
        process.wait().context("Waiting for target")?;

        anyhow::ensure!(
            fs::exists(llvm_profile_raw).context("Checking LLVM raw profile data")?,
            "LLVM raw profile data not found."
        );
        Ok(())
    }

    fn run_llvm_cov(&self, llvm_profile_data: String) -> Result<lcov::Report, anyhow::Error> {
        let mut child = Command::new("llvm-cov")
            .args([
                "export",
                &self.executable.to_string_lossy(),
                "--instr-profile",
                &llvm_profile_data,
                "--format=lcov",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Spawn llvm-cov export to lcov")?;

        let stdout = child.stdout.take().expect("We set it to pipe");
        let lcov_reader = lcov::Reader::new(BufReader::new(stdout));
        let report = lcov::Report::from_reader(lcov_reader).context("Parsing lcov report")?;
        let llvm_cov_status = child.wait().context("Waiting llvm-cov")?;
        anyhow::ensure!(llvm_cov_status.success(), "llvm-cov failed");
        Ok(report)
    }
}
