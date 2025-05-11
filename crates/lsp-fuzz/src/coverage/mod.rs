use std::{
    fs::{self},
    io::{self, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::Context;
use derive_new::new as New;
use tempfile::TempDir;

const RAW_COVERAGE_DATA_FILE_EXT: &str = "profraw";
const MERGED_COVERAGE_DATA_FILE_EXT: &str = "profdata";

#[derive(Debug, New)]
pub struct CoverageDataGenerator {
    executable: PathBuf,
    args: Vec<String>,
}

impl CoverageDataGenerator {
    /// /// Measures the coverage for a given input.
    ///
    /// This function takes an input and a converter to convert it to bytes, and measures the coverage
    /// by executing the target program with the input. The coverage data is stored at the specified path.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to measure coverage for
    /// * `converter` - Converter to convert the input to bytes
    /// * `coverage_data_path` - Path where the coverage data will be stored
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the coverage measurement is successful, or an error if something goes wrong.
    pub fn input_coverage_measure(&self, input_bytes: &[u8]) -> anyhow::Result<lcov::Report> {
        let temp_dir = TempDir::new().context("Creating tempdir")?;

        let llvm_profile_raw = format!(
            "{}/coverage.{}",
            temp_dir.path().display(),
            RAW_COVERAGE_DATA_FILE_EXT
        );
        let working_dir = temp_dir.path().join("working_dir");
        fs::create_dir_all(&working_dir).context("Creating working dir")?;
        let mut process = Command::new(&self.executable)
            .args(&self.args)
            .current_dir(working_dir)
            .env("LLVM_PROFILE_FILE", &llvm_profile_raw)
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

        let llvm_profile_data = format!(
            "{}/coverage.{}",
            temp_dir.path().display(),
            MERGED_COVERAGE_DATA_FILE_EXT
        );
        Command::new("llvm-profdata")
            .args(["merge", "-sparse"])
            .arg(llvm_profile_raw)
            .arg("-o")
            .arg(&llvm_profile_data)
            .status()
            .context("Running llvm-profdata")?;

        // Export coverage data to lcov format
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
